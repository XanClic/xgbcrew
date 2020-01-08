pub trait SaveState {
    fn export<T: std::io::Write>(&self, stream: &mut T);
    fn import<T: std::io::Read>(&mut self, stream: &mut T);
}


impl<U: serde::ser::Serialize + serde::de::DeserializeOwned> SaveState for U {
    fn export<T: std::io::Write>(&self, stream: &mut T) {
        bincode::serialize_into(stream, self).unwrap();
    }

    fn import<T: std::io::Read>(&mut self, stream: &mut T) {
        *self = bincode::deserialize_from(stream).unwrap();
    }
}
