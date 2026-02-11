# Data files installation

# IBus component file (dynamically configured in PHASE 2)
ibus_component_dir = datadir / 'ibus' / 'component'
if get_option('install_ibus_component')
  conf_data = configuration_data()
  conf_data.set('IBUS_ENGINE_PATH', get_option('prefix') / get_option('libexecdir') / 'emoji-input-engine')
  
  configure_file(
    input: 'ibus-component.xml',
    output: 'emoji-input.xml',
    configuration: conf_data,
    install_dir: ibus_component_dir
  )
endif

# Desktop file
if get_option('install_desktop_file')
  install_data('org.example.EmojiInputPrefs.desktop',
    install_dir: datadir / 'applications'
  )
endif

# GSettings schema (will be populated in PHASE 6)
if get_option('install_gschema')
  install_data('org.example.EmojiInput.gschema.xml',
    install_dir: datadir / 'glib-2.0' / 'schemas'
  )
endif

# Emoji database
install_data('emojis.json',
  install_dir: datadir / project_name
)
