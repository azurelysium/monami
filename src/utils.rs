extern crate base64;
extern crate crypto;
use crypto::{symmetriccipher, buffer, aes, blockmodes};
use crypto::buffer::{ ReadBuffer, WriteBuffer, BufferResult };
use sha2::{Sha512, Digest};


pub fn aes_encrypt(data: &str, secret: &str) -> Result<String, symmetriccipher::SymmetricCipherError> {
    let data_bytes = data.as_bytes();
    let iv: [u8; 16] = [0; 16];

    let mut hasher = Sha512::new();
    hasher.input(secret);
    let key = hasher.result();

    let mut encryptor = aes::cbc_encryptor(
        aes::KeySize::KeySize256,
        &key,
        &iv,
        blockmodes::PkcsPadding);

    let mut final_result = Vec::<u8>::new();
    let mut read_buffer = buffer::RefReadBuffer::new(&data_bytes);
    let mut buffer = [0; 4096];
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true).unwrap();
        final_result.extend(write_buffer.take_read_buffer().take_remaining().iter().copied());

        match result {
            BufferResult::BufferUnderflow => break,
            BufferResult::BufferOverflow => { }
        }
    }

    Ok(base64::encode(&final_result))
}

pub fn aes_decrypt(encrypted_data: &str, secret: &str) -> Result<String, symmetriccipher::SymmetricCipherError> {
    let data_bytes = match base64::decode(&encrypted_data) {
        Ok(decoded) => decoded,
        Err(_) => return Err(symmetriccipher::SymmetricCipherError::InvalidLength),
    };
    let iv: [u8; 16] = [0; 16];

    let mut hasher = Sha512::new();
    hasher.input(secret);
    let key = hasher.result();

    let mut decryptor = aes::cbc_decryptor(
        aes::KeySize::KeySize256,
        &key,
        &iv,
        blockmodes::PkcsPadding);

    let mut final_result = Vec::<u8>::new();
    let mut read_buffer = buffer::RefReadBuffer::new(&data_bytes);
    let mut buffer = [0; 4096];
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result = decryptor.decrypt(&mut read_buffer, &mut write_buffer, true)?;
        final_result.extend(write_buffer.take_read_buffer().take_remaining().iter().copied());
        match result {
            BufferResult::BufferUnderflow => break,
            BufferResult::BufferOverflow => { }
        }
    }

    Ok(String::from_utf8(final_result).unwrap())
}
