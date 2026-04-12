use anyhow::Result;
use clap::{Args, Subcommand};

mod aseprite;

pub trait GhostImporter {
    fn run(&self) -> Result<()>;
}

#[derive(Subcommand)]
pub enum ImportCommand {
    /// Import an exported Aseprite spritesheet, pack directory, or raw .ase file
    Aseprite(AsepriteImportArgs),
}

#[derive(Args, Clone, Debug)]
pub struct AsepriteImportArgs {
    /// Path to an exported spritesheet, Aseprite pack directory, or .ase file
    pub source: String,
    /// Name for the imported ghost
    #[arg(long)]
    pub name: String,
    /// Optional output path for the imported ghost (defaults to ~/.medium/ghosts/<name>)
    #[arg(long)]
    pub path: Option<String>,
    /// Specific exported sheet filename to use when the source directory has multiple sheets
    #[arg(long)]
    pub sheet: Option<String>,
    /// Short description for the imported ghost
    #[arg(long)]
    pub description: Option<String>,
    /// Artist or creator name
    #[arg(long)]
    pub artist: Option<String>,
    /// Required attribution text or credit request
    #[arg(long)]
    pub attribution: Option<String>,
    /// License or usage notes copied from the source pack
    #[arg(long = "license-notes")]
    pub license_notes: Option<String>,
    /// Additional importer notes
    #[arg(long)]
    pub notes: Option<String>,
    /// Width of a single frame in the source sheet
    #[arg(long, default_value_t = 24)]
    pub frame_width: u32,
    /// Height of a single frame in the source sheet
    #[arg(long, default_value_t = 24)]
    pub frame_height: u32,
    /// Number of leading frames to extract into the initial idle strip
    #[arg(long, default_value_t = 4)]
    pub idle_frames: u32,
    /// Frames per second for the imported idle animation
    #[arg(long, default_value_t = 8)]
    pub fps: u32,
}

pub fn run(command: ImportCommand) -> Result<()> {
    build_importer(command)?.run()
}

fn build_importer(command: ImportCommand) -> Result<Box<dyn GhostImporter>> {
    match command {
        ImportCommand::Aseprite(args) => Ok(Box::new(aseprite::AsepriteImporter::new(args))),
    }
}
