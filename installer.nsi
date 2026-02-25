; Joplin Smart Search — NSIS installer script
; Installs to %LocalAppData%\Programs\Joplin Smart Search (no admin rights needed)
; Run: makensis installer.nsi

Unicode true

!define APP_NAME    "Joplin Smart Search"
!define APP_EXE     "joplin-smart-search.exe"
!define APP_VERSION "0.1.0"
!define PUBLISHER   "Joplin Smart Search"
!define INSTALL_DIR "$LOCALAPPDATA\Programs\${APP_NAME}"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
!define SRC_EXE     "src-tauri/target/x86_64-pc-windows-msvc/release/${APP_EXE}"

Name            "${APP_NAME}"
OutFile         "src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/${APP_NAME}_${APP_VERSION}_x64-setup.exe"
InstallDir      "${INSTALL_DIR}"
RequestExecutionLevel user   ; No admin required

!include "MUI2.nsh"
!define MUI_ABORTWARNING

; Finish page: offer to launch the app immediately after install
!define MUI_FINISHPAGE_RUN          "$INSTDIR\${APP_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT     "Launch ${APP_NAME}"

; Pages
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; ── Install ──────────────────────────────────────────────────────────────────

Section "Install"
    SetOutPath "$INSTDIR"
    File "${SRC_EXE}"

    ; Desktop shortcut
    CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"

    ; Start menu shortcut
    CreateDirectory "$SMPROGRAMS\${APP_NAME}"
    CreateShortcut  "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${APP_EXE}"
    CreateShortcut  "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk"   "$INSTDIR\Uninstall.exe"

    ; Register uninstaller
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName"          "${APP_NAME}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion"       "${APP_VERSION}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher"            "${PUBLISHER}"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString"      "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation"      "$INSTDIR"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "NoModify"             "1"
    WriteRegStr HKCU "${UNINSTALL_KEY}" "NoRepair"             "1"
SectionEnd

; ── Uninstall ────────────────────────────────────────────────────────────────

Section "Uninstall"
    Delete "$INSTDIR\${APP_EXE}"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir  "$INSTDIR"

    Delete "$DESKTOP\${APP_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk"
    RMDir  "$SMPROGRAMS\${APP_NAME}"

    DeleteRegKey HKCU "${UNINSTALL_KEY}"
SectionEnd
