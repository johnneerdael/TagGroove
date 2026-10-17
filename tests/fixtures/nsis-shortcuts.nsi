Unicode true
SilentInstall silent
AutoCloseWindow true
RequestExecutionLevel user
Name "TagGroove shortcut fixtures"
OutFile "@OUTFILE@"
!include LogicLib.nsh
!include "Win\COM.nsh"
!include "Win\Propkey.nsh"
!define PRODUCTNAME "TagGroove"
!define INSTALLIDENTITY "OpenTag"
!define LEGACYMAINBINARY "opentag.exe"
!define MAINBINARYNAME "taggroove"
!define BUNDLEID "org.opentag.editor.e2e.shortcuts"
!define STARTMENUFOLDER "@STARTMENUFOLDER@"
Var PassiveMode
Var UpdateMode
Var NoShortcutMode
Var WixMode
Var OldMainBinaryName
Var AppStartMenuFolder

@UTILITY_MACROS@
@MIGRATION_MACRO@
@CLEANUP_MACRO@
@START_MENU_FUNCTION@
@DESKTOP_FUNCTION@

!macro AssertTarget LINK TARGET
  !insertmacro IsShortcutTarget "${LINK}" "${TARGET}"
  Pop $0
  ${If} $0 != 1
    FileOpen $9 "$EXEDIR\failure.txt" w
    FileWrite $9 "Unexpected shortcut target: ${LINK} -> ${TARGET}"
    FileClose $9
    SetErrorLevel 1
    Quit
  ${EndIf}
!macroend

!macro AssertMissing PATH
  ${If} ${FileExists} "${PATH}"
    FileOpen $9 "$EXEDIR\failure.txt" w
    FileWrite $9 "Unexpected remaining path: ${PATH}"
    FileClose $9
    SetErrorLevel 1
    Quit
  ${EndIf}
!macroend

!macro Case NAME
  StrCpy $INSTDIR "$EXEDIR\${NAME}"
  CreateDirectory "$INSTDIR\menu\FixtureFolder"
  CreateDirectory "$INSTDIR\desktop"
  SetOutPath $INSTDIR
  StrCpy $AppStartMenuFolder "FixtureFolder"
  StrCpy $OldMainBinaryName "opentag.exe"
  StrCpy $PassiveMode 0
  StrCpy $UpdateMode 0
  StrCpy $NoShortcutMode 0
  StrCpy $WixMode 0
  FileOpen $9 "$INSTDIR\opentag.exe" w
  FileWrite $9 "old fixture, never executed"
  FileClose $9
  FileOpen $9 "$INSTDIR\taggroove.exe" w
  FileWrite $9 "new fixture, never executed"
  FileClose $9
  FileOpen $9 "$INSTDIR\other.exe" w
  FileWrite $9 "unrelated fixture, never executed"
  FileClose $9
!macroend

!macro RunMigration
@INSTALL_MIGRATION@
!macroend

!macro RunCleanup
@UNINSTALL_CLEANUP@
!macroend

Section
  ; Existing desktop links must migrate without the finish-page callback.
  !insertmacro Case "unchecked-desktop"
  CreateShortcut "$INSTDIR\menu\OpenTag.lnk" "$INSTDIR\opentag.exe"
  CreateShortcut "$INSTDIR\menu\FixtureFolder\OpenTag.lnk" "$INSTDIR\opentag.exe"
  CreateShortcut "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\opentag.exe"
  !insertmacro RunMigration
  Call CreateOrUpdateStartMenuShortcut
  !insertmacro AssertTarget "$INSTDIR\menu\TagGroove.lnk" "$INSTDIR\taggroove.exe"
  !insertmacro AssertTarget "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\taggroove.exe"
  !insertmacro AssertMissing "$INSTDIR\menu\OpenTag.lnk"
  !insertmacro AssertMissing "$INSTDIR\desktop\OpenTag.lnk"
  !if "${STARTMENUFOLDER}" != ""
    !insertmacro AssertTarget "$INSTDIR\menu\FixtureFolder\TagGroove.lnk" "$INSTDIR\taggroove.exe"
    !insertmacro AssertMissing "$INSTDIR\menu\FixtureFolder\OpenTag.lnk"
  !endif

  ; Subsequent creation must not overwrite an unrelated destination link.
  !insertmacro Case "destination-collision"
  CreateShortcut "$INSTDIR\menu\OpenTag.lnk" "$INSTDIR\opentag.exe"
  CreateShortcut "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\opentag.exe"
  CreateShortcut "$INSTDIR\menu\TagGroove.lnk" "$INSTDIR\other.exe"
  CreateShortcut "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\other.exe"
  !insertmacro RunMigration
  Call CreateOrUpdateStartMenuShortcut
  Call CreateOrUpdateDesktopShortcut
  !insertmacro AssertTarget "$INSTDIR\menu\TagGroove.lnk" "$INSTDIR\other.exe"
  !insertmacro AssertTarget "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\other.exe"
  !insertmacro AssertTarget "$INSTDIR\menu\OpenTag.lnk" "$INSTDIR\taggroove.exe"
  !insertmacro AssertTarget "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\taggroove.exe"
  ; Update-mode uninstall preserves links; full uninstall removes owned aliases.
  StrCpy $UpdateMode 1
  !insertmacro RunCleanup
  !insertmacro AssertTarget "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\taggroove.exe"
  StrCpy $UpdateMode 0
  !insertmacro RunCleanup
  !insertmacro AssertMissing "$INSTDIR\menu\OpenTag.lnk"
  !insertmacro AssertMissing "$INSTDIR\desktop\OpenTag.lnk"
  !insertmacro AssertTarget "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\other.exe"

  ; Neither migration nor uninstall owns an unrelated legacy-labelled link.
  !insertmacro Case "unrelated-legacy"
  StrCpy $NoShortcutMode 1
  CreateShortcut "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\other.exe"
  !insertmacro RunMigration
  !insertmacro RunCleanup
  Call CreateOrUpdateDesktopShortcut
  !insertmacro AssertTarget "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\other.exe"
  !insertmacro AssertMissing "$INSTDIR\desktop\TagGroove.lnk"

  ; Both destination ownership cases exercise register-clobbering COM calls.
  !insertmacro Case "destination-owned-current"
  CreateShortcut "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\opentag.exe"
  CreateShortcut "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\taggroove.exe"
  !insertmacro RunMigration
  !insertmacro AssertMissing "$INSTDIR\desktop\OpenTag.lnk"
  !insertmacro AssertTarget "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\taggroove.exe"
  !insertmacro Case "destination-owned-legacy"
  CreateShortcut "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\opentag.exe"
  CreateShortcut "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\opentag.exe"
  !insertmacro RunMigration
  !insertmacro AssertMissing "$INSTDIR\desktop\OpenTag.lnk"
  !insertmacro AssertTarget "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\taggroove.exe"

  ; A retained label can migrate on a later update once its conflict disappears.
  !insertmacro Case "previous-collision-cleared"
  CreateShortcut "$INSTDIR\desktop\OpenTag.lnk" "$INSTDIR\taggroove.exe"
  !insertmacro RunMigration
  !insertmacro AssertMissing "$INSTDIR\desktop\OpenTag.lnk"
  !insertmacro AssertTarget "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\taggroove.exe"

  ; No-shortcut mode must not create shortcuts that were absent before.
  !insertmacro Case "fresh-no-shortcuts"
  StrCpy $NoShortcutMode 1
  !insertmacro RunMigration
  Call CreateOrUpdateStartMenuShortcut
  Call CreateOrUpdateDesktopShortcut
  !insertmacro AssertMissing "$INSTDIR\menu\TagGroove.lnk"
  !insertmacro AssertMissing "$INSTDIR\menu\FixtureFolder\TagGroove.lnk"
  !insertmacro AssertMissing "$INSTDIR\desktop\TagGroove.lnk"

  ; Fresh normal installs retain the configured Start Menu folder behavior.
  !insertmacro Case "fresh-with-shortcuts"
  !insertmacro RunMigration
  Call CreateOrUpdateStartMenuShortcut
  Call CreateOrUpdateDesktopShortcut
  !if "${STARTMENUFOLDER}" != ""
    !insertmacro AssertTarget "$INSTDIR\menu\FixtureFolder\TagGroove.lnk" "$INSTDIR\taggroove.exe"
  !else
    !insertmacro AssertTarget "$INSTDIR\menu\TagGroove.lnk" "$INSTDIR\taggroove.exe"
  !endif
  !insertmacro AssertTarget "$INSTDIR\desktop\TagGroove.lnk" "$INSTDIR\taggroove.exe"

  FileOpen $9 "$EXEDIR\result.txt" w
  FileWrite $9 "ok"
  FileClose $9
  SetErrorLevel 0
SectionEnd
