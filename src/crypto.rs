use std::time::{UNIX_EPOCH, Duration};

use radix64::STD as base64;
use tide::{Status, StatusCode};

use crate::server::JsonRequest;
use std::ops::Add;

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Packet {
    symmetric_key: String,
    nonce: String,
    message_block: String,
    signature: String,
    time_stamp: String,
}

impl Packet {
    pub fn to_json_request<T: serde::de::DeserializeOwned>(&self, privkey: &botan::Privkey, pubkey: &botan::Pubkey) -> tide::Result<T> {
        let time_stamp: u64 = self.time_stamp.parse()
            .status(StatusCode::RequestTimeout)?;
        if UNIX_EPOCH.add(Duration::from_secs(time_stamp)).elapsed().status(StatusCode::RequestTimeout)
            .map(|x|x.as_secs() > 30)? {
            return Err(tide::Error::from_str(StatusCode::RequestTimeout, "Time limit exceeded"))
        }
        let verifier = botan::Verifier::new(pubkey, "PKCS1v15(SHA-256)")
            .map_err(|_| tide::Error::from_str(StatusCode::Unauthorized, "Failed to Initialized Verifier"))?;
        let decrypter = botan::Decryptor::new(privkey, "OAEP(SHA-512)")
            .map_err(|_| tide::Error::from_str(StatusCode::Unauthorized, "Failed to Initialized Decrypter"))?;
        let symmetric_key = base64.decode(self.symmetric_key.as_bytes())
            .status(StatusCode::UnprocessableEntity)?;
        let signature = base64.decode(self.signature.as_bytes())
            .status(StatusCode::UnprocessableEntity)?;
        let message = base64.decode(self.message_block.as_bytes())
            .status(StatusCode::UnprocessableEntity)?;
        let nonce = base64.decode(self.nonce.as_bytes())
            .status(StatusCode::UnprocessableEntity)?;
        let aead_key = decrypter.decrypt(symmetric_key.as_slice())
            .map_err(|_| tide::Error::from_str(StatusCode::Unauthorized, "Invalid AEAD Key"))?;
        verifier.update(self.time_stamp.as_bytes())
            .map_err(|_| tide::Error::from_str(StatusCode::Unauthorized, "Unable to Update Verifier"))?;
        if let Ok(true) = verifier.finish(signature.as_slice()) {
            let aead = botan::Cipher::new("AES-256/GCM", botan::CipherDirection::Decrypt)
                .map_err(|_| tide::Error::from_str(StatusCode::Unauthorized, "Failed to Initialize Aead"))?;
            aead.set_key(aead_key.as_slice())
                .map_err(|_| tide::Error::from_str(StatusCode::Unauthorized, "Invalid AEAD Key"))?;
            aead.process(nonce.as_slice(), message.as_slice())
                .map_err(|_| tide::Error::from_str(StatusCode::Unauthorized, "Unable to process AEAD Message"))
                .and_then(|mut x| simd_json::from_slice(x.as_mut_slice())
                    .status(StatusCode::UnprocessableEntity))
        } else {
            Err(tide::Error::from_str(StatusCode::Unauthorized, "Signature Verification Error"))
        }
    }

    #[allow(unused)]
    pub fn from_json_request(res: JsonRequest, privkey: &botan::Privkey, pubkey: &botan::Pubkey) -> anyhow::Result<Self> {
        let mut aead_key = [0; 32];
        let mut nonce = [0; 12];
        let random = botan::RandomNumberGenerator::new_system()
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        random.fill(&mut aead_key);
        random.fill(&mut nonce);
        let message = simd_json::to_vec(&res)?;
        let aead = botan::Cipher::new("AES-256/GCM", botan::CipherDirection::Encrypt)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        aead.set_key(aead_key.as_ref())
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let message = aead.process(nonce.as_ref(), message.as_slice())
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let encryptor = botan::Encryptor::new(pubkey, "OAEP(SHA-512)")
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let time_stamp = std::time::SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?.as_secs().to_string();
        let signer = botan::Signer::new(privkey, "PKCS1v15(SHA-256)")
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        signer.update(time_stamp.as_ref())
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let aead_key = encryptor.encrypt(aead_key.as_ref(), &random)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let signature = signer.finish(&random)
            .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        Ok(Packet {
            symmetric_key: base64.encode(aead_key.as_slice()),
            signature: base64.encode(signature.as_slice()),
            message_block: base64.encode(message.as_slice()),
            nonce: base64.encode(nonce.as_ref()),
            time_stamp,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::crypto::Packet;
    use crate::server::JsonRequest;

    #[test]
    fn test() {
        let privk = botan::Privkey::load_encrypted_pem(include_str!("/code/keys/keeper_pri.pem"), "778878ZZzz").unwrap();
        let pubk = botan::Pubkey::load_pem(include_str!("/code/keys/keeper_pub.pem")).unwrap();
        let message = Packet::from_json_request(
            JsonRequest::ListPosts,
            &privk, &pubk,
        ).unwrap();
        println!("{:#?}", message);
        println!("{:?}", message.to_json_request::<crate::server::JsonRequest>(&privk, &pubk));
    }
}

