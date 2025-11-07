use eframe::egui::Ui;
use mhtame::{edit::{Edit, RszEditCtx}, rsz::{dump::RszDump, rszserde::{DeRsz, DeRszInstance, RszEditSerializerCtx}}};

pub struct Editor {
    display_string: String
}

impl Editor {
    pub fn show(ui: &mut Ui, dersz: &mut DeRsz) {
        for root in &dersz.roots {
            //let (root_hash, root_struct) = &dersz.structs[*root as usize];
            //let val = dersz.structs.get_mut(idx).ok_or(RszError::InvalidRszObjectIndex(self.idx, self.hash))?;

            let (hash, mut field_values) = {
                let val = dersz.structs.get_mut(*root as usize).unwrap();
                let (hash, field_values) = std::mem::take(&mut *val);
                (hash, field_values)
            };
            let root_type = RszDump::get_struct(hash).unwrap();
            let mut ctx = RszEditCtx::new(*root, &mut dersz.structs);
            field_values.edit(ui, &mut ctx);
        }
    }
}
