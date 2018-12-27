use super::*;

#[allow(dead_code)]
pub struct SubWork {
    pub midstate: Bytes,
    pub data2: Bytes,
    pub block_header: Bytes,
    pub nonce: Option<Bytes>,
}

#[allow(dead_code)]
impl SubWork {
    pub fn send_to_asic(&self) {
        unimplemented!();
    }

    pub fn diff(&self) -> BigUint {
        static NUM: [u32; 7] = [0xffffffffu32; 7];
        let mut temp = Bytes::new();
        temp.extend(&self.block_header);
        temp.extend(self.nonce.as_ref().unwrap());
        BigUint::from_slice(&NUM) / BigUint::from_bytes_be(flip32(temp).as_ref())
    }

    pub fn recv_nonce(&mut self, nonce: Bytes) {
        self.nonce = Some(nonce);
    }
}
