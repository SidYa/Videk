; Videk NSIS hooks: remove the per-user associations written by Settings > Асоціації файлів
; (src-tauri/src/assoc.rs) when the program is uninstalled.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Videk"
  DeleteRegValue HKCU "Software\RegisteredApplications" "Videk"
  DeleteRegKey HKCU "Software\Classes\Videk.Video"
  DeleteRegKey HKCU "Software\Classes\Videk.Audio"
  DeleteRegKey HKCU "Software\Classes\Applications\videk.exe"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
