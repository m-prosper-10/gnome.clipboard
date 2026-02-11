# Rust prefs build using Cargo
cargo = find_program('cargo', required: true)

# Determine build profile
if get_option('buildtype') == 'release'
  cargo_flags = ['--release']
  cargo_target_dir = 'release'
else
  cargo_flags = []
  cargo_target_dir = 'debug'
endif

# Build and install the Rust prefs app
prefs_binary = custom_target('emoji-input-prefs',
  output: 'emoji-input-prefs',
  build_by_default: true,
  build_always_stale: true,
  console: true,
  command: [
    'sh', '-c',
    'cd "' + meson.current_source_dir() + '" && ' +
    cargo.full_path() + ' build ' + ' '.join(cargo_flags) + ' && ' +
    'cp target/' + cargo_target_dir + '/emoji-input-prefs "' + meson.current_build_dir() + '/emoji-input-prefs"'
  ],
  install: true,
  install_dir: bindir,
)
