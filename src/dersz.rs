/*
impl RszValue {
    pub fn to_buffer(&self, base_addr: usize) -> Result<Vec<u8>> {
        let mut data: Vec<u8> = vec![];
        let struct_info = RszDump::rsz_map().get(self.hash().unwrap()).unwrap();
        for (i, field) in self.fields.iter().enumerate() {
            let field_info = &struct_info.fields[i];
            if field_info.array {
                if (data.len() + base_addr) % 4 as usize != 0 {
                    data.extend(vec![0; 4 - (data.len() + base_addr) % 4 as usize]);
                }
            }
            field.write_to(&mut data, &field_info, base_addr, field_info.array)?;
        }
        Ok(data)
    }
}*/

