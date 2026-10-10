#[macro_export]
macro_rules! define_symbols_set {
    ($($field:ident : $nerd:expr $(, $extra:expr)* ;)+) => {
        pub struct SymbolsSet {
            $(pub $field: &'static str,)+
        }

        impl Symbols {
            pub const NF: SymbolsSet = SymbolsSet {
                $($field: $nerd,)+
            };
            pub const PL: SymbolsSet = SymbolsSet {
                $($field: $crate::define_symbols_set!(@pl $($extra),*),)+
            };
            pub const PLAIN: SymbolsSet = SymbolsSet {
                $($field: $crate::define_symbols_set!(@plain $($extra),*),)+
            };
        }
    };

    (@pl) => { "" };
    (@pl $a:expr $(, $rest:expr)*) => { $a };

    (@plain) => { "" };
    (@plain $a:expr) => { "" };
    (@plain $a:expr, $b:expr $(, $rest:expr)*) => { $b };
}
