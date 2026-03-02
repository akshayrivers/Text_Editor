#[derive(Default, Eq, PartialEq, Debug)]
pub struct Message {
    pub msg: String,
}
impl Message {
    pub fn default_message(&mut self) {
        self.msg = String::from("HELP: Ctrl-S = save | Ctrl-Q = quit");
    }
}
