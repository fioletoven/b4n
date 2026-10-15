#[macro_export]
macro_rules! define_symbols_set {
    ($($field:ident : $nerd:literal $(, $extra:literal)* ;)+) => {
        pub struct SymbolsSet {
            $(pub $field: $crate::symbols::Symbol,)+
        }

        impl Symbols {
            pub const NF: SymbolsSet = SymbolsSet {
                $($field: $crate::define_symbols_set!(@make $nerd),)+
            };
            pub const PL: SymbolsSet = SymbolsSet {
                $($field: $crate::define_symbols_set!(@pl $($extra),*),)+
            };
            pub const PLAIN: SymbolsSet = SymbolsSet {
                $($field: $crate::define_symbols_set!(@plain $($extra),*),)+
            };
        }
    };

    (@make $c:literal) => {
        $crate::symbols::Symbol { ch: $c, raw: concat!($c), right: concat!($c, ' ') }
    };

    (@none) => {
        $crate::symbols::Symbol { ch: ' ', raw: "", right: "" }
    };

    (@pl) => { $crate::define_symbols_set!(@none) };
    (@pl $a:literal $(, $rest:literal)*) => { $crate::define_symbols_set!(@make $a) };

    (@plain) => { $crate::define_symbols_set!(@none) };
    (@plain $a:literal) => { $crate::define_symbols_set!(@none) };
    (@plain $a:literal, $b:literal $(, $rest:literal)*) => { $crate::define_symbols_set!(@make $b) };
}
