; PCTwin: Windows installer hooks (Tauri NSIS).
;
; A normal install asks once, in the installer's language, whether PCTwin may receive things
; from the person's other laptop; yes adds one narrow firewall rule: PCTwin's own program only,
; inbound TCP, private networks only (never public). No, a silent or a passive install adds
; nothing (Windows then asks itself, which the app explains first). Uninstalling always removes it.
; The words are written ahead of time; the rule's name is fixed so it is found again.

!define PCTWIN_FIREWALL_RULE "PCTwin (private networks)"

!macro PCTWIN_FIREWALL_WORDS
  ${If} $LANGUAGE == 1033
    StrCpy $R9 "Let PCTwin receive things from your other laptop?$\n$\nPCTwin will accept connections only on networks you have marked as private, such as your home Wi-Fi. Never on public Wi-Fi.$\n$\nIf you say no, Windows will ask you the first time you move."
  ${ElseIf} $LANGUAGE == 1036
    StrCpy $R9 "Autoriser PCTwin à recevoir des éléments de votre autre ordinateur portable ?$\n$\nPCTwin acceptera les connexions uniquement sur les réseaux que vous avez marqués comme privés, comme le Wi-Fi de votre domicile. Jamais sur un Wi-Fi public.$\n$\nSi vous refusez, Windows vous le demandera lors de votre premier déménagement."
  ${ElseIf} $LANGUAGE == 1034
    StrCpy $R9 "¿Permitir que PCTwin reciba cosas de tu otro portátil?$\n$\nPCTwin solo aceptará conexiones en las redes que hayas marcado como privadas, como el wifi de tu casa. Nunca en un wifi público.$\n$\nSi dices que no, Windows te lo preguntará la primera vez que hagas una mudanza."
  ${ElseIf} $LANGUAGE == 2070
    StrCpy $R9 "Permitir que o PCTwin receba coisas do seu outro portátil?$\n$\nO PCTwin só aceitará ligações nas redes que marcou como privadas, como o Wi-Fi de casa. Nunca numa rede Wi-Fi pública.$\n$\nSe disser que não, o Windows irá perguntar-lhe na primeira mudança."
  ${ElseIf} $LANGUAGE == 1046
    StrCpy $R9 "Permitir que o PCTwin receba coisas do seu outro notebook?$\n$\nO PCTwin só vai aceitar conexões nas redes que você marcou como privadas, como o Wi-Fi de casa. Nunca em Wi-Fi público.$\n$\nSe você disser não, o Windows vai perguntar na primeira vez que você fizer uma mudança."
  ${ElseIf} $LANGUAGE == 1025
    StrCpy $R9 "هل تسمح لـ PCTwin باستلام الأشياء من حاسوبك المحمول الآخر؟$\n$\nلن يقبل PCTwin الاتصالات إلا على الشبكات التي حددتها على أنها خاصة، مثل شبكة Wi-Fi في منزلك. ولن يقبلها أبدًا على شبكة Wi-Fi عامة.$\n$\nإذا رفضت، فسيسألك Windows في أول مرة تنقل فيها أشياءك."
  ${ElseIf} $LANGUAGE == 2052
    StrCpy $R9 "允许 PCTwin 接收来自您另一台笔记本电脑的内容吗？$\n$\nPCTwin 只会在您标记为专用的网络（例如家里的 Wi-Fi）上接受连接，绝不会在公共 Wi-Fi 上接受。$\n$\n如果您选择否，Windows 会在您第一次搬家时询问您。"
  ${ElseIf} $LANGUAGE == 1081
    StrCpy $R9 "क्या PCTwin को आपके दूसरे लैपटॉप से चीज़ें लेने दें?$\n$\nPCTwin केवल उन्हीं नेटवर्क पर कनेक्शन स्वीकार करेगा जिन्हें आपने निजी के रूप में चिह्नित किया है, जैसे आपके घर का Wi-Fi। सार्वजनिक Wi-Fi पर कभी नहीं।$\n$\nअगर आप मना करते हैं, तो पहली बार सामान ले जाते समय Windows आपसे पूछेगा।"
  ${Else}
    StrCpy $R9 "Let PCTwin receive things from your other laptop?$\n$\nPCTwin will accept connections only on networks you have marked as private, such as your home Wi-Fi. Never on public Wi-Fi.$\n$\nIf you say no, Windows will ask you the first time you move."
  ${EndIf}
!macroend

; PCTwin's own update and uninstall wait for the engine's safe stopping point (Task List 2.9).
; While PCTwin is in the middle of a step that must not be cut part way (moving files, installing
; apps and settings, placing app data, an undo), it holds safe-stop.lock in its local app data
; folder open with no sharing, so FileOpen here fails; the file is free again the moment the
; step ends.
;
; Every account's PCTwin is looked at, not only the one that started this installer: the
; installer is for everyone and runs elevated (an administrator's password typed over the
; person's shoulder, SYSTEM from a management tool), so its own $LOCALAPPDATA is not the person's.
; Each profile in the system's list (HKLM ProfileList) is checked. A file that cannot be opened
; for any reason counts as busy: PCTwin is never closed while the answer is unknown.
;
; Between finding every PCTwin at a safe point and closing them, no step may start: the
; installer first makes safe-stop-closing beside the program and keeps it open, then checks
; every account again, and only then closes PCTwin. A first step in PCTwin takes its own file and
; then looks for this one (held: it does not start), so one of the two always sees the other.
;
; A normal run waits up to a minute, then asks Retry or Cancel. A silent or passive run waits the
; same minute and then gives up (the install or removal is cancelled; nothing is closed).
!ifndef PCTWIN_SAFE_STOP_FILE
  !define PCTWIN_SAFE_STOP_FILE "com.creoleailabs.pctwin\safe-stop.lock"
!endif
!ifndef PCTWIN_CLOSING_FILE
  !define PCTWIN_CLOSING_FILE "safe-stop-closing"
!endif
; The system's list of profiles (a test build can point this elsewhere).
!ifndef PCTWIN_PROFILES_ROOT
  !define PCTWIN_PROFILES_ROOT HKLM
!endif
!ifndef PCTWIN_PROFILES_KEY
  !define PCTWIN_PROFILES_KEY "SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList"
!endif
; How often, and how long apart (milliseconds), the installer looks again before it asks.
!ifndef PCTWIN_SAFE_TRIES
  !define PCTWIN_SAFE_TRIES 30
!endif
!ifndef PCTWIN_SAFE_PAUSE
  !define PCTWIN_SAFE_PAUSE 2000
!endif

!macro PCTWIN_BUSY_WORDS
  ${If} $LANGUAGE == 1036
    StrCpy $R9 "PCTwin est en train de terminer une étape de votre déménagement (fichiers, applications ou annulation).$\n$\nAttendez qu'elle se termine, ou arrêtez le déménagement dans PCTwin, puis choisissez Réessayer. Rien de ce qui est déjà copié et vérifié n'est perdu."
  ${ElseIf} $LANGUAGE == 1034
    StrCpy $R9 "PCTwin está terminando un paso de tu mudanza (archivos, aplicaciones o deshacer).$\n$\nEspera a que termine, o detén la mudanza en PCTwin, y elige Reintentar. No se pierde nada de lo que ya está copiado y comprobado."
  ${ElseIf} $LANGUAGE == 2070
    StrCpy $R9 "O PCTwin está a terminar um passo da sua mudança (ficheiros, aplicações ou anular).$\n$\nAguarde que termine, ou pare a mudança no PCTwin, e escolha Tentar novamente. Nada do que já foi copiado e verificado se perde."
  ${ElseIf} $LANGUAGE == 1046
    StrCpy $R9 "O PCTwin está terminando uma etapa da sua mudança (arquivos, aplicativos ou desfazer).$\n$\nEspere terminar, ou pare a mudança no PCTwin, e escolha Tentar novamente. Nada do que já foi copiado e verificado é perdido."
  ${ElseIf} $LANGUAGE == 1025
    StrCpy $R9 "ينهي PCTwin الآن خطوة من نقل أشيائك (الملفات أو التطبيقات أو التراجع).$\n$\nانتظر حتى تنتهي، أو أوقف النقل في PCTwin، ثم اختر إعادة المحاولة. لن يضيع شيء مما نُسخ وتم التحقق منه."
  ${ElseIf} $LANGUAGE == 2052
    StrCpy $R9 "PCTwin 正在完成搬家中的一个步骤（文件、应用或撤消）。$\n$\n请等它完成，或在 PCTwin 中停止搬家，然后选择“重试”。已经复制并检查过的内容不会丢失。"
  ${ElseIf} $LANGUAGE == 1081
    StrCpy $R9 "PCTwin आपके सामान ले जाने का एक चरण पूरा कर रहा है (फ़ाइलें, ऐप या पहले जैसा करना)।$\n$\nइसके पूरा होने तक रुकें, या PCTwin में ले जाना रोकें, फिर फिर से कोशिश करें चुनें। जो पहले से कॉपी और जाँचा जा चुका है, वह नहीं खोएगा।"
  ${Else}
    StrCpy $R9 "PCTwin is finishing a step of your move (files, apps or an undo).$\n$\nWait for it to finish, or stop the move in PCTwin, then choose Retry. Nothing already copied and checked is lost."
  ${EndIf}
!macroend

; Whether any account's PCTwin is in the middle of a step: $R6 = 1 busy (or not known), 0 when
; every account's file is absent or free. Opened to read only, for an instant. Uses $R1 to $R4.
!macro PCTWIN_ANY_BUSY_FUNCTION PREFIX
  Function ${PREFIX}PCTwinAnyBusy
    StrCpy $R6 0
    StrCpy $R3 0
    ${If} ${RunningX64}
      SetRegView 64
    ${EndIf}
    pctwin_busy_next:
      ClearErrors
      EnumRegKey $R2 ${PCTWIN_PROFILES_ROOT} "${PCTWIN_PROFILES_KEY}" $R3
      ${If} ${Errors}
      ${AndIf} $R3 == 0
        ; The list itself cannot be read: not known.
        StrCpy $R6 1
        Goto pctwin_busy_end
      ${EndIf}
      StrCmp $R2 "" pctwin_busy_end
      IntOp $R3 $R3 + 1
      ReadRegStr $R1 ${PCTWIN_PROFILES_ROOT} "${PCTWIN_PROFILES_KEY}\$R2" "ProfileImagePath"
      StrCmp $R1 "" pctwin_busy_next
      ExpandEnvStrings $R1 "$R1"
      StrCpy $R1 "$R1\AppData\Local\${PCTWIN_SAFE_STOP_FILE}"
      IfFileExists "$R1" 0 pctwin_busy_next
      ClearErrors
      FileOpen $R4 "$R1" r
      ${If} ${Errors}
        StrCpy $R6 1
        Goto pctwin_busy_end
      ${EndIf}
      FileClose $R4
      Goto pctwin_busy_next
    pctwin_busy_end:
    ${If} ${RunningX64}
      SetRegView lastused
    ${EndIf}
  FunctionEnd
!macroend
!insertmacro PCTWIN_ANY_BUSY_FUNCTION ""
!insertmacro PCTWIN_ANY_BUSY_FUNCTION "un."

; Waits for every account's PCTwin to be at a safe stopping point, then keeps any from leaving it:
; $R7 = 1 with safe-stop-closing made and held open in $R0 (close it with PCTWIN_CLOSED once
; PCTwin is closed), or $R7 = 0 when the person chose Cancel (or, in a silent or passive run, it
; never became safe), with nothing held.
!macro PCTWIN_WAIT_SAFE_STOP PREFIX
  StrCpy $R7 1
  StrCpy $R5 0
  pctwin_safe_try:
    Call ${PREFIX}PCTwinAnyBusy
    StrCmp $R6 1 pctwin_safe_busy
    ; Free a moment ago: from now on no first step may start, then every account is looked at
    ; again (a step that started in between is seen now).
    CreateDirectory "$INSTDIR"
    ClearErrors
    FileOpen $R0 "$INSTDIR\${PCTWIN_CLOSING_FILE}" w
    IfErrors pctwin_safe_busy
    Call ${PREFIX}PCTwinAnyBusy
    StrCmp $R6 1 0 pctwin_safe_done
    FileClose $R0
    Delete "$INSTDIR\${PCTWIN_CLOSING_FILE}"
  pctwin_safe_busy:
    IntOp $R5 $R5 + 1
    ${If} $R5 < ${PCTWIN_SAFE_TRIES}
      Sleep ${PCTWIN_SAFE_PAUSE}
      Goto pctwin_safe_try
    ${EndIf}
    !insertmacro PCTWIN_BUSY_WORDS
    MessageBox MB_RETRYCANCEL|MB_ICONINFORMATION "$R9" /SD IDCANCEL IDRETRY pctwin_safe_retry
    StrCpy $R7 0
    Goto pctwin_safe_done
  pctwin_safe_retry:
    StrCpy $R5 0
    Goto pctwin_safe_try
  pctwin_safe_done:
!macroend

; PCTwin is closed: the installer's file goes (PCTwin started again later finds nothing held).
!macro PCTWIN_CLOSED
  FileClose $R0
  Delete "$INSTDIR\${PCTWIN_CLOSING_FILE}"
!macroend

; A test build can look at what happens just before PCTwin is closed.
!ifmacrondef PCTWIN_BEFORE_CLOSE
  !macro PCTWIN_BEFORE_CLOSE
  !macroend
!endif

!macro NSIS_HOOK_PREINSTALL
  ; A PCTwin still open (in the tray, or another person's on this laptop) holds its own files, and
  ; the installer then stopped at "Error opening file for writing" (first real test, 28 October
  ; 2026). Close PCTwin and its helpers first, for every account; each by its exact name, never a
  ; pattern that could match this installer. A move that was running picks up again from its
  ; journal the next time PCTwin opens. First, wait for every PCTwin's safe stopping point.
  !insertmacro PCTWIN_WAIT_SAFE_STOP ""
  ${If} $R7 != 1
    Abort
  ${EndIf}
  !insertmacro PCTWIN_BEFORE_CLOSE
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "${MAINBINARYNAME}.exe"'
  Pop $R8
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "pctwin-helper.exe"'
  Pop $R8
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "pctwin-handover.exe"'
  Pop $R8
  Sleep 500
  !insertmacro PCTWIN_CLOSED
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; An earlier version's rule is replaced, never doubled.
  nsExec::Exec '"$SYSDIR\netsh.exe" advfirewall firewall delete rule name="${PCTWIN_FIREWALL_RULE}"'
  Pop $R8
  ${IfNot} ${Silent}
  ${AndIf} $PassiveMode != 1
    !insertmacro PCTWIN_FIREWALL_WORDS
    MessageBox MB_YESNO|MB_ICONQUESTION "$R9" /SD IDNO IDNO pctwin_no_rule
    nsExec::Exec '"$SYSDIR\netsh.exe" advfirewall firewall add rule name="${PCTWIN_FIREWALL_RULE}" dir=in action=allow program="$INSTDIR\${MAINBINARYNAME}.exe" protocol=TCP profile=private enable=yes'
    Pop $R8
    pctwin_no_rule:
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro PCTWIN_WAIT_SAFE_STOP "un."
  ${If} $R7 != 1
    Abort
  ${EndIf}
  ; Closed while the installer's file is held, as for an install, so no step starts in between.
  !insertmacro PCTWIN_BEFORE_CLOSE
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "${MAINBINARYNAME}.exe"'
  Pop $R8
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "pctwin-helper.exe"'
  Pop $R8
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "pctwin-handover.exe"'
  Pop $R8
  Sleep 500
  !insertmacro PCTWIN_CLOSED
  nsExec::Exec '"$SYSDIR\netsh.exe" advfirewall firewall delete rule name="${PCTWIN_FIREWALL_RULE}"'
  Pop $R8
!macroend
