use anyhow::anyhow;
use anyhow::Context;
use clap::Parser;
use options::Options;
use slog::{info, Drain, Logger};
mod options;
mod video;
mod ui;
mod input;
mod emu;

use std::num;

fn main() -> anyhow::Result<()> {
    let options = Options::parse();

    let decorator = slog_term::TermDecorator::new().stdout().build();
    let drain = slog_term::CompactFormat::new(decorator)
        .use_custom_timestamp(|_: &mut dyn std::io::Write| Ok(()))
        .build()
        .fuse();
    let logger = Logger::root(
        slog_async::Async::new(drain)
            .overflow_strategy(slog_async::OverflowStrategy::Block)
            .thread_name("async logger".to_owned())
            .build()
            .fuse(),
        slog::o!(),
    );

    info!(logger, "Initializing SDL2 context");
    let sdl_context = sdl2::init().map_err(anyhow::Error::msg)?;

    let controller_subsystem = sdl_context.game_controller().map_err(anyhow::Error::msg)?;

    // let controller_mappings =
    //     include_str!("../../../external/SDL_GameControllerDB/gamecontrollerdb.txt");
    // controller_subsystem.load_mappings_from_read(&mut Cursor::new(controller_mappings))?;

    // let num_joysticks = controller_subsystem.num_joysticks().map_err(anyhow::Error::msg)?;
    // let available_controllers = (0..num_joysticks)
    //     .filter(|&id| controller_subsystem.is_game_controller(id))
    //     .collect::<Vec<u32>>();
    //
    // let mut active_controller = match available_controllers.first() {
    //     Some(&id) => {
    //         let controller = controller_subsystem.open(id)?;
    //         info!(logger, "Found game controller: {}", controller.name());
    //         Some(controller)
    //     }
    //     _ => {
    //         info!(logger, "No game controllers were found");
    //         None
    //     }
    // };

    let mut renderer = video::init(&sdl_context)?;
    // let (audio_interface, mut _sdl_audio_device) = audio::create_audio_player(&sdl_context)?;
    // let mut rom_name = opts.rom_name();

    // let bios_bin = load_bios(&opts.bios);

    // let mut gba = Box::new(GameBoyAdvance::new(
    //     bios_bin.clone(),
    //     opts.cartridge_from_opts()?,
    //     audio_interface,
    // ));

    Ok(())
}
