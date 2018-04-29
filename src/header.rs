use httpcodec::HeaderField;

#[derive(Debug)]
pub enum Connection {
    Close,
}
impl From<Connection> for HeaderField<'static, 'static> {
    fn from(_: Connection) -> Self {
        unsafe { HeaderField::new_unchecked("Connection", "close") }
    }
}
