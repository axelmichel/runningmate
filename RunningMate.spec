# -*- mode: python ; coding: utf-8 -*-


a = Analysis(
    ['runningmate.py'],
    pathex=[],
    binaries=[],
    datas=[('splash_screen.png', '.')],
    hiddenimports=[],
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
    name='runningmate',
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
    icon=['icon.png'],
)
app = BUNDLE(
    exe,
    name='runningmate.app',
    icon='icon.png',
    bundle_identifier=None,
)
