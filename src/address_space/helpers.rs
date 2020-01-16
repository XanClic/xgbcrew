pub trait U8Split {
    fn split_into_u8<F: FnMut(u16, u8)>(self, base_addr: u16, func: F);
    fn construct_from_u8<F: FnMut(u16) -> u8>(base_addr: u16, func: F)
        -> Self;
}

impl U8Split for u8 {
    fn split_into_u8<F: FnMut(u16, u8)>(self, base_addr: u16, mut func: F) {
        func(base_addr, self);
    }

    fn construct_from_u8<F: FnMut(u16) -> u8>(base_addr: u16, mut func: F)
        -> Self
    {
        func(base_addr)
    }
}

impl U8Split for u16 {
    fn split_into_u8<F: FnMut(u16, u8)>(self, base_addr: u16, mut func: F) {
        func(base_addr, self as u8);
        func(base_addr.wrapping_add(1u16), (self >> 8) as u8);
    }

    fn construct_from_u8<F: FnMut(u16) -> u8>(base_addr: u16, mut func: F)
        -> Self
    {
        (func(base_addr) as u16) |
            ((func(base_addr.wrapping_add(1u16)) as u16) << 8)
    }
}
