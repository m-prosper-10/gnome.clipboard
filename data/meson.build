# Data files installation

# IBus component file (will be populated in PHASE 2)
ibus_component_dir = datadir / 'ibus' / 'component'
if get_option('install_ibus_component')
  install_data('ibus-component.xml',
    install_dir: ibus_component_dir,
    rename: 'emoji-input.xml'
  )
endif

# Desktop file (will be populated in PHASE 7)
if get_option('install_desktop_file')
  install_data('org.example.EmojiInput.desktop',
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
