pub trait SaveState {
    fn export<T: std::io::Write>(&self, stream: &mut T, version: u64);
    fn import<T: std::io::Read>(&mut self, stream: &mut T, version: u64);
}


impl<U: serde::ser::Serialize + serde::de::DeserializeOwned> SaveState for U {
    fn export<T: std::io::Write>(&self, stream: &mut T, _version: u64) {
        bincode::serialize_into(stream, self).unwrap();
    }

    fn import<T: std::io::Read>(&mut self, stream: &mut T, _version: u64) {
        *self = bincode::deserialize_from(stream).unwrap();
    }
}


pub fn export_root<U: SaveState, V: std::io::Write>
                  (obj: &U, mut stream: &mut V, version: u64)
{
    if version > 0 {
        /* xgbc save state file */
        bincode::serialize_into(&mut stream, &0x9bc54fe57473f11eu64).unwrap();
        bincode::serialize_into(&mut stream, &version).unwrap();
    }

    SaveState::export(obj, stream, version);
}

pub fn import_root<U: SaveState, V: std::io::Read + std::io::Seek>
                  (obj: &mut U, mut stream: &mut V, max_version: u64)
{
    let magic: u64 = bincode::deserialize_from(&mut stream).unwrap();

    let version: u64 =
        if magic == 0x9bc54fe57473f11eu64 {
            bincode::deserialize_from(&mut stream).unwrap()
        } else {
            stream.seek(std::io::SeekFrom::Start(0)).unwrap();
            0u64
        };

    if version > max_version {
        panic!("Save state version ({}) unsupported (maximum supported \
                version: {})", version, max_version);
    }

    SaveState::import(obj, stream, version);
}
