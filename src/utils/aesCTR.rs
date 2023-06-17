use std::num::NonZeroU32;
use hex_literal::hex;
use aes::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use ring::digest::{SHA256, Digest};
use ring::pbkdf2::{derive, PBKDF2_HMAC_SHA256};
use ring::pbkdf2;
use aes::Aes128;
use md5::{Md5, Digest as MDigest};

type Aes128Ctr64BE = ctr::Ctr64BE<aes::Aes128>;

pub struct AesCTR {
    pub password: String,
    pub sizeSalt: String,
    pub passwdOutward:String,
    pub key:[u8;16],
    pub iv:[u8;16],
    pub source_iv:[u8;16],
    pub cipher:Aes128Ctr64BE,
}

impl AesCTR {
    pub fn new(password: &str,size_salt:&str) -> Self {
        let salt = "AES-CTR".as_bytes();
        let iterations =NonZeroU32::new(1000).unwrap();;
        let output_length = 16;
        let mut out = vec![0; output_length];
        pbkdf2::derive(
            PBKDF2_HMAC_SHA256,
            iterations,
            &salt,
            password.as_bytes(),
            &mut out,
        );
        let hex_result = hex::encode(out);
        let passwdSalt = format!("{}{}",hex_result,size_salt);
        let mut hasher = Md5::new();
        hasher.update(passwdSalt);
        let key:[u8;16] = hasher.clone().finalize().into();

        let mut ivhasher = Md5::new();
        ivhasher.update(size_salt.to_string());
        let mut iv:[u8;16] = ivhasher.finalize().into();
        let mut cipher = Aes128Ctr64BE::new(&key.into(), &iv.into());
        AesCTR {
            password: password.to_string(),
            sizeSalt: size_salt.to_string(),
            passwdOutward:hex_result,
            key,
            iv,
            cipher,
            source_iv:iv.clone(),
        }
    }

    pub fn decrypt(&mut self, mut buf:&mut Vec<u8>){
        self.cipher.apply_keystream(&mut buf)
    }

    pub fn set_position(&mut self, position: usize) {
        let increment = position / 16;
        self.increment_iv(increment as u32);
        self.cipher = Aes128Ctr64BE::new(&self.key.into(), &self.iv.into());
    }

    fn increment_iv(&mut self, increment: u32) {
        const MAX_UINT32: u32 = 0xffffffff;
        let increment_big: u32 = increment / MAX_UINT32;
        let increment_little: u32 = (increment % MAX_UINT32) - increment_big;
        let mut overflow: u32 = 0;
        for idx in 0..4 {
            let mut num = u32::from_be_bytes([
                self.iv[12 - idx*4],
                self.iv[13 - idx*4],
                self.iv[14 - idx*4],
                self.iv[15 - idx*4],
            ]);
            let mut inc: u32 = overflow;
            if (idx == 0){inc += increment_little;};
            if (idx == 1){inc += increment_big;};
            num += inc;
            let num_big: u32 = num / MAX_UINT32;
            let num_little = (num % MAX_UINT32) - num_big;
            overflow = num_big;
            self.iv[12 - idx*4..16 - idx*4].copy_from_slice(&num_little.to_be_bytes());
        }
    }

}