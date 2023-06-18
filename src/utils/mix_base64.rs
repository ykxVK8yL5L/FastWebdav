use std::collections::BTreeMap;
use sha2::{Sha256 as MSha256,Digest as MDigest};

const SOURCE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-~+";

fn init_ksa(passwd: &str) -> String {
    let passwd = passwd.as_bytes(); // 假设已有密码
    let mut hasher = MSha256::new();
    hasher.update(passwd);
    let key = hasher.finalize();
    
    let mut k = BTreeMap::new();
    let mut sbox = BTreeMap::new();
    let source_key: Vec<char> = SOURCE.chars().collect();

    // 对S表进行初始赋值
    for i in 0..source_key.len() {
        sbox.insert(i, i as u8);
    }

    // 用种子密钥对K表进行填充
    for i in 0..source_key.len() {
        let val_index = i % key.len();
        let val = key[val_index];
        k.insert(i, val as u8);
    }
   
    // 对S表进行置换
    let mut j: usize = 0;
    for i in 0..source_key.len() {
        let is = *sbox.get(&i).unwrap() as usize;
        let ks = *k.get(&i).unwrap() as usize;
        j = (j + is + ks) % source_key.len();
        let temp = is;
        sbox.insert(i, *sbox.get(&j).unwrap() as u8);
        sbox.insert(j, temp as u8);
    }
    let mut secret = String::new();
    for value in sbox.values() {
        secret.push(source_key[*value as usize]);
    }
    secret
}



pub struct MixBase64 {
    pub secret: String,
}

impl MixBase64 {
    pub fn new(password: &str) -> Self {
        let passwd_salt = format!("{}mix64",password); 
        let secret_ksa = init_ksa(&passwd_salt);
        let secret = if password.len() == 64 { password } else { &secret_ksa };
        MixBase64 {
            secret: secret.to_string(),
        }
    }

    // pub fn encode(&self,password:&str) -> String {
    //     let buffer: &[u8] = password.as_bytes();
    //     let CHARS:Vec<char>= self.secret.chars().collect();
    //     let mut result = String::new();
    //     let mut arr: &[u8]=&buffer[0..];
    //     let mut bt: [u8; 3] = [0, 0, 0];
    //     let mut char: String;

    //     for i in (0..buffer.len()).step_by(3) {
    //         if i + 3 > buffer.len() {
    //             arr = &buffer[i..];
    //             break;
    //         }
    //         bt = [buffer[i], buffer[i+1], buffer[i+2]];
    //         char = format!("{}{}{}{}", 
    //                 CHARS[(bt[0] >> 2) as usize],
    //                 CHARS[ (((bt[0] & 3) << 4) | (bt[1] >> 4)) as usize],
    //                 CHARS[ (((bt[1] & 15) << 2) | (bt[2] >> 6)) as usize],
    //                 CHARS[(bt[2] & 63) as usize]);
    //         result.push_str(&char);
    //     }
    //     if buffer.len() % 3 == 1 {
    //         char = format!("{}{}{}{}", 
    //                 CHARS[(arr[0] >> 2) as usize],
    //                 CHARS[((arr[0] & 3) << 4) as usize],
    //                 CHARS[64],
    //                 CHARS[64]);
    //         result.push_str(&char);
    //     } else if buffer.len() % 3 == 2 {
    //         char = format!("{}{}{}{}", 
    //                 CHARS[(arr[0] >> 2) as usize],
    //                 CHARS[ (((arr[0] & 3) << 4) | (arr[1] >> 4)) as usize],
    //                 CHARS[((arr[1] & 15) << 2) as usize],
    //                 CHARS[64]);
    //         result.push_str(&char);
    //     }
    //     return result;
    // }


    pub fn decode(&self,base64_str: &str) -> String {
        //let secret=&self.secret;
        let chars:Vec<char>= self.secret.chars().collect();
        let mut map_chars = BTreeMap::new();
        for (index, element) in chars.iter().enumerate() {
            map_chars.insert(*element, index);
        }
        // let mut size = (base64_str.len() / 4) * 3;
        // let mut j = 0;
        // if let Some(_) = base64_str.find(&format!("{}{}", secret.chars().nth(64).unwrap(), secret.chars().nth(64).unwrap())) {
        //     size -= 2;
        // } else if let Some(_) = base64_str.find(secret.chars().nth(64).unwrap()) {
        //     size -= 1;
        // }
   
        let mut buffer: Vec<u8> = Vec::new();
        let mut i: usize = 0;
    
        while i < base64_str.len() {
            let enc1: u8 = *map_chars.get(&base64_str.chars().nth(i).unwrap()).unwrap() as u8;
            let enc2: u8 = *map_chars.get(&base64_str.chars().nth(i+1).unwrap()).unwrap() as u8;
            let enc3: u8 = *map_chars.get(&base64_str.chars().nth(i+2).unwrap()).unwrap() as u8;
            let enc4: u8 = *map_chars.get(&base64_str.chars().nth(i+3).unwrap()).unwrap() as u8;
            buffer.push((enc1 << 2) | (enc2 >> 4));
            if enc3 != 64 {
                buffer.push(((enc2 & 15) << 4) | (enc3 >> 2));
            }
            if enc4 != 64 {
                buffer.push(((enc3 & 3) << 6) | enc4);
            }
            i += 4;
        }
        std::str::from_utf8(&buffer).unwrap().to_owned()
    }

}