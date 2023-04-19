#[macro_export]
macro_rules! read {
    ($self:ident, $($buf:tt)*) => {
        $self.controller
            .read(&$self.volume, &mut $self.config_file, &mut $self.$($buf)*)
    };
}
