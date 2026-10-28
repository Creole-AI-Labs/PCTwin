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

!macro NSIS_HOOK_PREINSTALL
  ; A PCTwin still open (in the tray, or another person's on this laptop) holds its own files, and
  ; the installer then stopped at "Error opening file for writing" (first real test, 28 October
  ; 2026). Close PCTwin and its helpers first, for every account; each by its exact name, never a
  ; pattern that could match this installer. A move that was running picks up again from its
  ; journal the next time PCTwin opens.
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "${MAINBINARYNAME}.exe"'
  Pop $R8
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "pctwin-helper.exe"'
  Pop $R8
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /t /im "pctwin-handover.exe"'
  Pop $R8
  Sleep 500
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
  nsExec::Exec '"$SYSDIR\netsh.exe" advfirewall firewall delete rule name="${PCTWIN_FIREWALL_RULE}"'
  Pop $R8
!macroend
