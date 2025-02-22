import os
import glob
from PyInstaller.utils.hooks import collect_submodules

# Automatically collect all .svg files from the icons folder
icon_folder = ('icons', 'icons')

a = Analysis(
    ['runningmate.py'],
    pathex=[],
    binaries=[],
    datas=[
        ('splash_screen.png', '.'),
        icon_folder,
    ],
    hiddenimports=collect_submodules('geopy') + ['PyQt6.QtSvgWidgets'],  # Collect all geopy submodules
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='RunningMate',
    debug=False,
    bootloader_ignore_signals=False,
    strip=True,
    upx=False,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=['app-icon.icns'],
)
app = BUNDLE(
    exe,
    name='RunningMate.app',
    icon='app-icon.icns',
    bundle_identifier=None,
)
