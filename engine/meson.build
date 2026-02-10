# Rust engine build using Cargo
cargo = find_program('cargo', required: true)
rustc = find_program('rustc', required: true)

# Determine build profile
if get_option('buildtype') == 'release'
  cargo_profile = 'release'
  cargo_flags = ['--release']
  target_dir = meson.current_source_dir() / 'target' / 'release'
else
  cargo_profile = 'debug'
  cargo_flags = []
  target_dir = meson.current_source_dir() / 'target' / 'debug'
endif

# Build the Rust engine
engine_binary = custom_target('emoji-input-engine',
  output: 'emoji-input-engine',
  build_by_default: true,
  build_always_stale: true,
  console: true,
  command: [
    cargo, 'build',
    cargo_flags,
    '--manifest-path', meson.current_source_dir() / 'Cargo.toml',
  ],
  install: true,
  install_dir: libexecdir,
)

# Create a dependency for the engine (for other subprojects if needed)
engine_dep = declare_dependency(
  sources: engine_binary,
)
