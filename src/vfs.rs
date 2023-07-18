use std::num::NonZeroU32;
use aes::cipher::{KeyIvInit, StreamCipher};
use ring::pbkdf2::{PBKDF2_HMAC_SHA256};
use ring::pbkdf2;
use md5::{Md5, Digest as MDigest};

type Aes128Ctr128BE = ctr::Ctr128BE<aes::Aes128>;



pub struct AesCTR {
    pub password: String,
    pub size_salt: String,
    pub passwd_outward:String,
    pub key:[u8;16],
    pub iv:[u8;16],
    pub source_iv:[u8;16],
    pub cipher:Aes128Ctr128BE,
}

impl AesCTR {
    pub fn new(password: &str,size_salt:&str) -> Self {
        let salt = "AES-CTR".as_bytes();
        let iterations =NonZeroU32::new(1000).unwrap();
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
        let passwd_salt = format!("{}{}",hex_result,size_salt);
        let mut hasher = Md5::new();
        hasher.update(passwd_salt);
        let key:[u8;16] = hasher.clone().finalize().into();

        let mut ivhasher = Md5::new();
        ivhasher.update(size_salt.to_string());
        let iv:[u8;16] = ivhasher.finalize().into();
        let cipher = Aes128Ctr128BE::new(&key.into(), &iv.into());
        AesCTR {
            password: password.to_string(),
            size_salt: size_salt.to_string(),
            passwd_outward:hex_result,
            key,
            iv,
            cipher,
            source_iv:iv.clone(),
        }
    }

    pub fn encrypt(&mut self, message_bytes: Vec<u8>)->Vec<u8>{
        let mut encrypted_buffer = message_bytes;
        self.cipher.apply_keystream(&mut encrypted_buffer);
        encrypted_buffer
    }
    pub fn decrypt(&mut self, cipher_bytes: Vec<u8>)->Vec<u8>{
        let mut decrypted_buffer = cipher_bytes;
        self.cipher.apply_keystream(&mut decrypted_buffer);
        decrypted_buffer
    }

    // pub fn decryptb2b(&mut self, content:Vec<u8>, mut buf:&mut Vec<u8>)->Result<(), cipher::StreamCipherError>{
    //     self.cipher.apply_keystream_b2b(&content,&mut buf)
    // }
   
    pub fn set_position(&mut self, position: usize) {
        let increment = position / 16;
        self.increment_iv(increment as u64);
        self.cipher = Aes128Ctr128BE::new(&self.key.into(), &self.iv.into());
        let offset = position%16;
        let buffer = vec![0u8; offset]; 
        self.encrypt(buffer);
    }

    // fn increment_iv(&mut self, increment: u32) {
    //     const MAX_UINT32: u32 = 0xffffffff;
    //     let increment_big: u32 = increment / MAX_UINT32;
    //     let increment_little: u32 = (increment % MAX_UINT32) - increment_big;
    //     let mut overflow: u32 = 0;
    //     for idx in 0..4 {
    //         let mut num = u32::from_be_bytes([
    //             self.iv[12 - idx*4],
    //             self.iv[13 - idx*4],
    //             self.iv[14 - idx*4],
    //             self.iv[15 - idx*4],
    //         ]);
    //         let mut inc: u32 = overflow;
    //         if idx == 0 {inc += increment_little;};
    //         if idx == 1 {inc += increment_big;};
    //         num += inc;
    //         let num_big: u32 = num / MAX_UINT32;
    //         let num_little = (num % MAX_UINT32) - num_big;
    //         overflow = num_big;
    //         self.iv[12 - idx*4..16 - idx*4].copy_from_slice(&num_little.to_be_bytes());
    //     }
    // }

    fn increment_iv(&mut self,increment: u64) {
        const MAX_UINT32: u64 = 0xffffffff;
        let increment_big = increment / MAX_UINT32;
        let increment_little = (increment % MAX_UINT32) - increment_big;
        let mut overflow = 0;
    
        for idx in 0..4 {
            let num = u32::from_be_bytes([
                self.iv[12 - idx * 4],
                self.iv[13 - idx * 4],
                self.iv[14 - idx * 4],
                self.iv[15 - idx * 4],
            ]);
    
            let mut inc = overflow;
            if idx == 0 {
                inc += increment_little;
            }
            if idx == 1 {
                inc += increment_big;
            }
    
            let mut num = u64::from(num);
            num = num.wrapping_add(inc);
    
            let num_big = num / MAX_UINT32;
            let num_little = (num % MAX_UINT32) - num_big;
            overflow = num_big;
    
            self.iv[12 - idx * 4] = ((num_little >> 24) & 0xff) as u8;
            self.iv[13 - idx * 4] = ((num_little >> 16) & 0xff) as u8;
            self.iv[14 - idx * 4] = ((num_little >> 8) & 0xff) as u8;
            self.iv[15 - idx * 4] = (num_little & 0xff) as u8;
        }
    }







}
