; Windows installer for the Oscyl desktop app, for the current user only (no
; admin rights). It copies the folder bundle-windows.sh made to
; %LOCALAPPDATA%\Programs\Oscyl, adds Oscyl to the Start menu and to
; Settings > Apps, where it is uninstalled. Settings and libraries stay.
;
;   makensis /DVERSION=X.Y.Z /DSRC=dist\Oscyl /DOUT=dist\oscyl-setup.exe oscyl.nsi
;
; For a release candidate, also pass /DNUMERIC=X.Y.Z (the file version
; Windows shows must be numbers only).

Unicode true
ManifestDPIAware true
RequestExecutionLevel user
SetCompressor /SOLID lzma

!include MUI2.nsh
!include FileFunc.nsh

!define KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Oscyl"

Name "Oscyl"
OutFile "${OUT}"
InstallDir "$LOCALAPPDATA\Programs\Oscyl"
InstallDirRegKey HKCU "${KEY}" "InstallLocation"
!ifndef NUMERIC
    !define NUMERIC "${VERSION}"
!endif
VIProductVersion "${NUMERIC}.0"
VIAddVersionKey /LANG=0 "ProductName" "Oscyl"
VIAddVersionKey /LANG=0 "FileDescription" "Oscyl installer"
VIAddVersionKey /LANG=0 "FileVersion" "${NUMERIC}"
VIAddVersionKey /LANG=0 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=0 "LegalCopyright" "MIT OR Apache-2.0"

!define MUI_ICON "${SRC}\oscyl.ico"
!define MUI_UNICON "${SRC}\oscyl.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\bin\Oscyl.exe"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
; The app's languages (po/LINGUAS); Windows picks the user's.
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Italian"

; What the installer owns. Only these are removed, never the whole folder:
; it may be one the user chose.
!macro RemoveFiles
    RMDir /r "$INSTDIR\bin"
    RMDir /r "$INSTDIR\share"
    RMDir /r "$INSTDIR\licences"
    Delete "$INSTDIR\LICENSE-MIT"
    Delete "$INSTDIR\LICENSE-APACHE"
    Delete "$INSTDIR\oscyl.ico"
!macroend

Section
    SetShellVarContext current
    ; An update replaces the old version's files instead of adding to them.
    !insertmacro RemoveFiles
    SetOutPath "$INSTDIR"
    File /r "${SRC}\*"
    WriteUninstaller "$INSTDIR\uninstall.exe"
    CreateShortCut "$SMPROGRAMS\Oscyl.lnk" "$INSTDIR\bin\Oscyl.exe" "" "$INSTDIR\oscyl.ico"

    WriteRegStr HKCU "${KEY}" "DisplayName" "Oscyl"
    WriteRegStr HKCU "${KEY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "${KEY}" "Publisher" "Oscyl"
    WriteRegStr HKCU "${KEY}" "DisplayIcon" "$INSTDIR\oscyl.ico"
    WriteRegStr HKCU "${KEY}" "URLInfoAbout" "https://github.com/oscyl/oscyl"
    WriteRegStr HKCU "${KEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "${KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKCU "${KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
    WriteRegDWORD HKCU "${KEY}" "NoModify" 1
    WriteRegDWORD HKCU "${KEY}" "NoRepair" 1
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    WriteRegDWORD HKCU "${KEY}" "EstimatedSize" $0
SectionEnd

Section "Uninstall"
    SetShellVarContext current
    Delete "$SMPROGRAMS\Oscyl.lnk"
    !insertmacro RemoveFiles
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"
    DeleteRegKey HKCU "${KEY}"
SectionEnd
