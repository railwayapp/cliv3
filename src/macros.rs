macro_rules! commands_enum {
    ($($module:tt),*) => (
      paste::paste! {
        #[derive(Subcommand)]
        pub(crate) enum Commands {
            $(
              [<$module:camel>]($module::Args),
            )*
        }

        impl Commands {
            #[allow(dead_code)]
            pub(crate) async fn exec(cli: Args) -> Result<()> {
              match cli.command {
                $(
                  Commands::[<$module:camel>](args) => $module::command(args, cli.json).await?,
                )*
              }
              Ok(())
            }
        }
      }
    );
}
