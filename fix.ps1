 = Get-Content -Raw 'd:\code\qidong\src-tauri\src\process.rs' 
 =  -replace 'use portable_pty::\{native_pty_system, CommandBuilder, PtySize\};', 'use portable_pty::{native_pty_system, CommandBuilder, Master, PtySize};' 
Set-Content -Path 'd:\code\qidong\src-tauri\src\process.rs' -Value  -NoNewline
