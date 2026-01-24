#[macro_export]
macro_rules! define_key_commands {
    (
        bindings = $bindings_path:path;

        $(#[$enum_attr:meta])*
        $vis:vis enum $enum_name:ident {
            $(
                $(#[$variant_attr:meta])*
                $variant:ident => $str_name:literal @ $($key:literal),+
            ),* $(,)?
        }
    ) => {
        $(#[$enum_attr])*
        $vis enum $enum_name {
            $(
                $(#[$variant_attr])*
                $variant
            ),*
        }

        impl $enum_name {
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $str_name),*
                }
            }
        }

        impl ::std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::std::str::FromStr for $enum_name {
            type Err = KeyCommandError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($str_name => Ok(Self::$variant)),*,
                    _ => Err(KeyCommandError::UnknownCommand),
                }
            }
        }

        impl Default for $bindings_path {
            fn default() -> Self {
                Self::empty()
                $(                                        // for each variant
                    $(.with($key, $enum_name::$variant))+ // for each key
                )*
            }
        }
    };
}
