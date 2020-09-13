use std::ops::Add;
use std::time::{Duration, UNIX_EPOCH};

use anyhow::*;
use radix64::STD as base64;
use serde::Serialize;
use tide::{Status, StatusCode};
use xactor::Context;

use crate::KeyPair;

const TIME_OUT: u64 = 30;

#[derive(Eq, Ord, PartialOrd, PartialEq, Debug)]
struct Stamp {
    time_stamp: u64,
    nonce: String,
}


#[xactor::message(result = "()")]
#[derive(Clone)]
struct CleanUp;

#[xactor::message(result = "bool")]
struct PutStamp(Stamp);

#[derive(Default)]
pub struct StampKeeper {
    stamps: std::collections::BTreeSet<Stamp>
}

#[async_trait::async_trait]
impl xactor::Actor for StampKeeper {
    async fn started(&mut self, ctx: &mut Context<Self>) -> xactor::Result<()> {
        Ok(ctx.send_interval(CleanUp, Duration::from_secs(TIME_OUT / 2)))
    }
}

#[async_trait::async_trait]
impl xactor::Handler<CleanUp> for StampKeeper {
    async fn handle(&mut self, _ctx: &mut Context<Self>, _msg: CleanUp) {
        while !self.stamps.is_empty()
            && UNIX_EPOCH.add(Duration::from_secs(self.stamps.first().unwrap().time_stamp))
            .elapsed().unwrap().as_secs() > TIME_OUT {
            self.stamps.pop_first();
        }
    }
}

#[async_trait::async_trait]
impl xactor::Handler<PutStamp> for StampKeeper {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: PutStamp) -> bool {
        if self.stamps.contains(&msg.0) {
            false
        } else {
            self.stamps.insert(msg.0);
            true
        }
    }
}


#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Packet {
    symmetric_key: String,
    nonce: String,
    message_block: String,
    signature: String,
    time_stamp: String,
}

impl Packet {
    pub async fn to_json_request<T: serde::de::DeserializeOwned>(
        &self,
        key_pair: &KeyPair,
        mut stamp_keeper: Option<&mut xactor::Addr<StampKeeper>>,
    ) -> anyhow::Result<T> {
        let time_stamp: u64 = self.time_stamp.parse()?;
        if UNIX_EPOCH.add(Duration::from_secs(time_stamp)).elapsed().status(StatusCode::RequestTimeout)
            .map_err(|x| anyhow!("{}", x))
            .map(|x| x.as_secs() > TIME_OUT)? {
            return Err(anyhow!("Time Limit Exceeded"));
        }
        if stamp_keeper.is_some() {
            let addr = stamp_keeper.as_mut().unwrap();
            match addr.call(PutStamp(Stamp { time_stamp, nonce: self.nonce.clone() })).await {
                Ok(false) | Err(_) => {
                    return Err(anyhow!("Stamp Validation Failed"));
                }
                _ => ()
            }
        }
        let verifier = botan::Verifier::new(&key_pair.owner_public, "PKCS1v15(SHA-256)")
            .map_err(|_| anyhow!("Verifier Initialization Failed"))?;
        let decrypter = botan::Decryptor::new(&key_pair.server_private, "OAEP(SHA-512)")
            .map_err(|_| anyhow!("Decryptor Initialization Failed"))?;
        let symmetric_key = base64.decode(self.symmetric_key.as_bytes())?;
        let signature = base64.decode(self.signature.as_bytes())?;
        let message = base64.decode(self.message_block.as_bytes())?;
        let nonce = base64.decode(self.nonce.as_bytes())?;
        let aead_key = decrypter.decrypt(symmetric_key.as_slice())
            .map_err(|_| anyhow!("Invalid AEAD Key"))?;
        verifier.update(self.time_stamp.as_bytes())
            .and_then(|_| verifier.update(aead_key.as_ref()))
            .and_then(|_| verifier.update(message.as_ref()))
            .and_then(|_| verifier.update(nonce.as_ref()))
            .map_err(|_| anyhow!("Verifier Update Error"))?;
        if let Ok(true) = verifier.finish(signature.as_slice()) {
            let aead = botan::Cipher::new("AES-256/GCM", botan::CipherDirection::Decrypt)
                .map_err(|_| anyhow!("AEAD Initialization Error"))?;
            aead.set_key(aead_key.as_slice())
                .map_err(|_| anyhow!("Invalid AEAD Key"))?;
            aead.process(nonce.as_slice(), message.as_slice())
                .map_err(|_| anyhow!("AEAD Process Error"))
                .and_then(|mut x| simd_json::from_slice(x.as_mut_slice())
                    .map_err(Into::into))
        } else {
            Err(anyhow!("Signature Verification Error"))
        }
    }

    #[allow(unused)]
    pub async fn from_json_request<T: Serialize>(res: T, key_pair: &KeyPair) -> anyhow::Result<Self> {
        let mut aead_key = [0; 32];
        let mut nonce = [0; 12];
        let privkey = &key_pair.server_private;
        let pubkey = &key_pair.owner_public;
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
            .and_then(|_| signer.update(aead_key.as_ref()))
            .and_then(|_| signer.update(message.as_ref()))
            .and_then(|_| signer.update(nonce.as_ref()))
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


    pub async fn from_json_request_tide<T: Serialize>(res: T, key_pair: &KeyPair) -> tide::Result<Packet> {
        Self::from_json_request(res, key_pair).await
            .map_err(|x| {
                tide::Error::from_str(StatusCode::InternalServerError, x)
            })
    }
}

#[cfg(test)]
mod test {
    use std::thread::sleep;
    use std::time::Duration;

    use crate::crypto::Packet;
    use crate::server::JsonRequest;

    extern crate test;

    #[async_std::test]
    async fn no_duplicate_request() {
        use xactor::Actor;
        let privk = botan::Privkey::load_encrypted_pem(include_str!("/code/keys/keeper_pri.pem"), "778878ZZzz").unwrap();
        let pubk = botan::Pubkey::load_pem(include_str!("/code/keys/keeper_pub.pem")).unwrap();
        let mut actor = super::StampKeeper::start_default().await;
        let message = Packet::from_json_request(
            JsonRequest::ListPosts,
            &privk, &pubk,
        ).unwrap();
        println!("{:#?}", message);
        println!("{:?}", message.to_json_request::<crate::server::JsonRequest>(&privk, &pubk,
                                                                               Some(&mut actor)).await.unwrap());
        assert!(message.to_json_request::<crate::server::JsonRequest>(&privk, &pubk,
                                                                      Some(&mut actor)).await.is_err());
    }

    #[async_std::test]
    async fn timeout_detection() {
        use xactor::Actor;
        let privk = botan::Privkey::load_encrypted_pem(include_str!("/code/keys/keeper_pri.pem"), "778878ZZzz").unwrap();
        let pubk = botan::Pubkey::load_pem(include_str!("/code/keys/keeper_pub.pem")).unwrap();
        let mut actor = super::StampKeeper::start_default().await;
        let message = Packet::from_json_request(
            JsonRequest::ListPosts,
            &privk, &pubk,
        ).unwrap();
        println!("{:#?}", message);
        async_std::task::sleep(Duration::from_secs(super::TIME_OUT + 1)).await;
        println!("{:?}", message.to_json_request::<crate::server::JsonRequest>(&privk, &pubk,
                                                                               Some(&mut actor)).await.is_err());
    }
}

