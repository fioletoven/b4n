#[macro_export]
macro_rules! define_icons_set {
    ($($field:ident : $nerd:expr $(, $extra:expr)* ;)+) => {
        pub struct IconsSet {
            $(pub $field: &'static str,)+
        }

        impl Icons {
            pub const NF: IconsSet = IconsSet {
                $($field: $nerd,)+
            };
            pub const PL: IconsSet = IconsSet {
                $($field: $crate::define_icons_set!(@pl $($extra),*),)+
            };
            pub const PLAIN: IconsSet = IconsSet {
                $($field: $crate::define_icons_set!(@plain $($extra),*),)+
            };
        }
    };

    (@pl) => { "" };
    (@pl $a:expr $(, $rest:expr)*) => { $a };

    (@plain) => { "" };
    (@plain $a:expr) => { "" };
    (@plain $a:expr, $b:expr $(, $rest:expr)*) => { $b };
}
