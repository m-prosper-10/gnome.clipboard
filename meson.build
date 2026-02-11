project('gnome-emoji-input', 'c',
  version: '0.1.0',
  license: 'MIT',
  meson_version: '>= 0.59.0',
  default_options: [
    'warning_level=2',
  ]
)

# Project metadata
project_name = meson.project_name()
project_version = meson.project_version()

# Installation paths
prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
libexecdir = prefix / get_option('libexecdir')

# Dependencies
ibus_dep = dependency('ibus-1.0', version: '>= 1.5.0')

# Subprojects
subdir('engine')
subdir('ui')
subdir('prefs')
subdir('data')

# Summary
summary({
  'prefix': prefix,
  'bindir': bindir,
  'datadir': datadir,
  'libexecdir': libexecdir,
}, section: 'Directories')

summary({
  'IBus': ibus_dep.version(),
}, section: 'Dependencies')
