use super::GuiLayout;

#[derive(Debug, Default)]
pub struct DisconnectedGuiLayout {

}

impl GuiLayout for DisconnectedGuiLayout {
    fn new() -> Self {
        Self { }
    }
}