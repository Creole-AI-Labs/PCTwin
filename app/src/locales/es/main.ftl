# PCTwin screens, Spanish (es). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = Inicio de PCTwin
language-label = Idioma
motion-label = Movimiento
motion-system = Movimiento: como el sistema
motion-reduced = Movimiento: reducido
motion-off = Movimiento: apagado

# The opening (Wordplay); its tagline is the home headline.
opening-label = PCTwin se está abriendo
opening-status = Abriendo PCTwin.
opening-skip = Saltar la intro
opening-tagline = Envía archivos o múdalo todo.

footer-free = Gratis. Sin cuenta.
footer-help = Ayuda

## F01 Home

welcome-title-line1 = Envía archivos
welcome-title-line2 = o múdalo todo.
welcome-body = Envía unos pocos archivos a un portátil cercano, o lleva tus archivos, aplicaciones y ajustes a tu portátil nuevo.
welcome-start = Empezar un traslado completo

## F01 Home: the cards, My history, a move to resume, files waiting

home-quick-send-body = Enviar o recibir archivos
home-move-body = Traslada tu configuración
home-restore-body = Recupérala
home-rescue-body = Salva lo importante
home-history = Mi historial
home-history-body = Tus traslados y sus informes
name-title = Ponle nombre a este portátil.
name-body = Los demás portátiles cercanos lo encontrarán con este nombre. Puedes cambiarlo más tarde, al emparejar.
name-use = Usar este nombre
home-update-title = Hay una nueva versión disponible.
home-update-action = Actualizar ahora
home-updating = Actualizando. PCTwin se cerrará y se volverá a abrir solo.
home-update-failed = No se pudo instalar la actualización. PCTwin funciona como antes; se te ofrecerá de nuevo la próxima vez.
home-update-busy = Termina primero tu traslado. Después actualiza PCTwin.
home-resume-body = Tu progreso está guardado.
home-resume-action = Reanudar
    .aria-label = Reanudar tu traslado
home-waiting =
    { NUMBER($count) ->
        [one] { $count } archivo esperando a { $to }
        [many] { $count } de archivos esperando a { $to }
       *[other] { $count } archivos esperando a { $to }
    }

## Header: this laptop, the language and motion control, Help

header-this-laptop = Tu portátil

## F06 Old or new laptop

role-back = Atrás
role-eyebrow = Empezar un traslado completo
role-title = ¿Qué portátil es este?
role-body = Abre PCTwin en los dos portátiles. Luego indica cuál es este.
role-old-title = Este es mi portátil antiguo
role-old-body = Enviar mis archivos, aplicaciones y ajustes.
role-new-title = Este es mi portátil nuevo
role-new-body = Recibir mi configuración desde otro portátil.
role-note = Nada se traslada hasta que revises y apruebes el plan. Tu portátil antiguo no cambia durante el traslado.

## Frame parts (footer note, Back, steps)

footer-note-move = Traslados personales gratis
footer-note-send = Envíos cercanos gratis
frame-back = Atrás
# Read out by screen readers only; the step shapes carry the meaning on screen.
frame-step-done = Hecho
frame-step-current = Ahora
frame-step-todo = Después
frame-step-position = Paso { $number } de { $total }

## F01 Unfinished move

resume-open = Reanudar tu traslado

## C01 How will you connect?

connect-eyebrow = Tu traslado
connect-title = ¿Cómo vas a conectarlos?
connect-body = Elijas la forma que elijas, los pasos son los mismos.
connect-wifi-title = Wi-Fi
connect-wifi-body = Los dos portátiles en la misma Wi-Fi.
connect-wifi-status = Disponible
connect-later = Un cable de red, el cable puente de PCTwin y el traslado mediante un disco externo llegarán en una versión posterior.
connect-usb-note = Un cable USB normal no puede conectar dos portátiles para un traslado.
connect-use-wifi = Usar Wi-Fi
# Added: sizes and speeds as measured, shown the same way on every connection screen.
connect-unit-mbps = { NUMBER($value, maximumFractionDigits: 0) } MB/s
connect-unit-gb = { NUMBER($value, maximumFractionDigits: 1) } GB

## C02 Put both laptops together

connect-prep-title = Pon los dos portátiles juntos.
connect-prep-body = Conéctalos a la misma Wi-Fi y abre PCTwin en cada uno.
connect-prep-step-1-title = Usa la misma Wi-Fi
connect-prep-step-1-body = Copiar archivos no necesita internet.
# $network is the Wi-Fi name this laptop is on.
connect-prep-step-1-network = Red: { $network }.
connect-prep-step-2-title = Permite encontrar portátiles cerca
connect-prep-step-2-body = Si Windows o macOS pide permiso, elige Permitir.
connect-prep-step-3-title = Elige un papel en cada portátil
connect-prep-step-3-body = Uno envía. El otro recibe.
connect-prep-note = Una Wi-Fi de invitados o pública puede impedir que los portátiles se vean.
connect-prep-continue = Seguir con la vinculación

## C06 Meet your other laptop

peer-eyebrow = Tu traslado
peer-title = Conoce tu otro portátil.
peer-body = Los dos portátiles están conectados y listos.
peer-sending = Envía
peer-receiving = Recibe
peer-old = Portátil antiguo
peer-new = Portátil nuevo
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
peer-status-update = Hay que actualizar
peer-status-unsupported = En espera
peer-ready-note = PCTwin muestra tus archivos solo después de vincular los dos portátiles. Si las versiones son distintas, actualiza PCTwin en los dos.
peer-version-differs = Estos portátiles tienen versiones distintas de PCTwin. Actualiza PCTwin en los dos para continuar.
peer-version-old-body = Otra versión: { $version }.
peer-version-new-body = Necesita la misma versión en los dos portátiles.
peer-update-title = Actualizar PCTwin
peer-update-body = Pon la misma versión en los dos portátiles.
# $from and $to are system names, for example "Windows 11" and "macOS". The mockup sentence does not use them.
peer-direction-unsupported = Esta versión de PCTwin todavía no puede trasladar entre estos dos sistemas.
peer-continue = Comprobar la conexión

## C07 Let’s check the connection

check-eyebrow = Tu traslado
check-title = Comprobemos la conexión.
check-body = PCTwin comprueba la velocidad, el espacio y la batería para planear el traslado y calcular el tiempo.
check-measuring = Midiendo la conexión…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = velocidad de conexión
check-space-label = libres en el portátil nuevo
check-link-weak = Débil, así que el traslado puede ir lento. Acercar los portátiles al router Wi-Fi puede ayudar.
check-power-title = Corriente
check-power-both = Los dos portátiles están enchufados.
check-power-battery = { $label } funciona con batería. Enchúfalo para que un traslado largo no se interrumpa.
check-space-title = Espacio
check-space-body = { $free } libres en el portátil nuevo. Antes de trasladar nada, verás si cabe lo que elegiste.
check-internet-title = Internet
check-internet-on = Conectado. Copiar archivos no necesita internet.
check-internet-off = Sin conexión. No pasa nada: copiar archivos no necesita internet.
check-status-ready = Listo
check-status-plug-in = Enchufar
check-status-connected = Disponible
check-status-not-needed = No hace falta
# Added: what is said when something could not be read, so it is never shown as fine.
check-status-unknown = Se desconoce
check-speed-unknown = PCTwin no pudo medir la conexión esta vez. Los archivos se mueven igual; solo falta la estimación de tiempo.
check-space-unknown = PCTwin no pudo leer el espacio libre del portátil nuevo.
check-power-unknown = PCTwin no pudo saber si { $label } está enchufado.
check-internet-unknown = PCTwin no pudo saber si este portátil está en línea. No pasa nada: copiar archivos no necesita internet.
check-again = Comprobar de nuevo
check-estimate-note = El tiempo calculado es aproximado, no una promesa.
check-continue = Elegir qué trasladar
check-continue-new = Continuar
# Added 23 October 2026: the short checklist both laptops show before the move.
check-before-title = Antes de que empiece el traslado
check-before-lids-title = Deja las dos tapas abiertas
check-before-lids-body = Una tapa cerrada puede dejar el portátil en reposo, y eso pausa el traslado.
check-before-near-title = Quédate cerca del router Wi-Fi
check-before-near-body = Deja los dos portátiles cerca de él mientras se trasladan tus cosas.

## C09 Waiting for your other laptop

reconnect-title = Esperando a tu otro portátil.
reconnect-body = Se conserva todo lo copiado y comprobado hasta ahora.
# Added: the title of the saved-progress card (the mockup shows Move paused by connection).
reconnect-card-title = Traslado en pausa por la conexión
reconnect-progress =
    .aria-label = Traslado en pausa por la conexión
# $done is an already formatted size, for example "81.6 GB". $total is passed but not shown.
reconnect-saved = { $done } copiados y comprobados hasta ahora. Falta el resto.
# $ago is already formatted, for example "45 seconds ago".
reconnect-last-seen = Última conexión: { $ago }.
reconnect-status-offline = Sin conexión
reconnect-status-trying = Reconectando
reconnect-status-pending = En espera
reconnect-trying = Intentando reconectar solo… Deja los dos portátiles encendidos, activos y en la misma Wi-Fi.
reconnect-expired = Ha pasado demasiado tiempo para reconectar solo. Vincúlalos de nuevo y PCTwin comprobará lo que ya se copió.
reconnect-unavailable = No se puede llegar a { $label }. Comprueba que esté encendido, activo, con PCTwin abierto y en la misma Wi-Fi.
reconnect-try-now = Reintentar la conexión
reconnect-pair-again = Vincular de nuevo
reconnect-nothing-new = Hace un rato que no llega nada nuevo. PCTwin sigue intentándolo solo, o puedes detener el traslado. Todo lo copiado y comprobado hasta ahora se conserva.
reconnect-details = Reconexión segura
reconnect-details-body = Solo estos dos portátiles pueden volver a conectarse, y cada vez usan una conexión privada nueva.

## C10 The connection has ended (dialog)

ended-title = La conexión ha terminado.
ended-body = Tu traslado está guardado. El código anterior ya no sirve.
ended-kept-title = Se conserva
ended-kept-body = El registro de tu traslado y todo lo copiado y comprobado.
ended-not-kept-title = No se conserva
ended-not-kept-body = El permiso para seguir sin vincular de nuevo.
ended-pair-again = Vincular de nuevo
ended-pair-again-body = Conéctalos otra vez y PCTwin comprobará lo que ya se copió.
ended-see-moved = Abrir el traslado guardado
ended-see-moved-body = Mira lo que se ha copiado y comprobado hasta ahora.
ended-home = Volver al inicio

## Pairing, shared

pair-back = Atrás
pair-try-again = Intentar de nuevo
pair-more-details = Más detalles
pair-heads-up-title = Tu portátil puede hacerte una pregunta.
pair-heads-up-local-network = Tu Mac te preguntará si PCTwin puede buscar dispositivos en tu red local. Elige Permitir para que tus dos portátiles puedan encontrarse.
pair-heads-up-firewall = Windows puede preguntarte si PCTwin puede aceptar conexiones. Elige Permitir para que tu portátil nuevo pueda llegar a este.
pair-heads-up-continue = Continuar

## C03 Old laptop: show the code

pair-old-eyebrow = Portátil antiguo
pair-old-title = Conectemos tus portátiles.
pair-old-body = En tu portátil nuevo, escribe el código de abajo.
pair-old-next = Después, elegirás un número que aparece en los dos portátiles.
pair-old-network = Deja los dos portátiles en la misma Wi-Fi.
pair-old-waiting = Esperando a tu portátil nuevo…
pair-old-starting = Preparando un código…
pair-old-label-title = Nombre de este portátil
pair-old-label-hint = Tu portátil nuevo encontrará este con este nombre.
pair-old-shuffle = Elegir otro nombre
pair-old-rename = Escribir un nombre
pair-old-rename-label = Nombre para este portátil
pair-old-rename-note = Cualquiera en esta Wi-Fi puede ver este nombre.
pair-old-rename-save = Usar este nombre
pair-old-rename-cancel = Cancelar
pair-old-code-meta =
    { $seconds ->
        [one] Cambia cada minuto. Nuevo código en { $seconds } segundo.
        [many] Cambia cada minuto. Nuevo código en { $seconds } de segundos.
       *[other] Cambia cada minuto. Nuevo código en { $seconds } segundos.
    }
pair-old-steps-title = En tu portátil nuevo
pair-old-step-1 = Abre PCTwin y elige Mi portátil nuevo.
pair-old-step-2 = Si aparece una lista, elige el nombre de este portátil.
pair-old-step-3 = Escribe el código de seis cifras que ves aquí.
pair-old-wait-title = Ya viene un código nuevo.
pair-old-wait-body =
    { $seconds ->
        [one] Ese intento no funcionó. Aparecerá un código nuevo en { $seconds } segundo.
        [many] Ese intento no funcionó. Aparecerá un código nuevo en { $seconds } de segundos.
       *[other] Ese intento no funcionó. Aparecerá un código nuevo en { $seconds } segundos.
    }
pair-old-locked-title = Alguien en esta red probó códigos incorrectos.
pair-old-locked-body = Tras cinco códigos incorrectos, la vinculación se detuvo para proteger este portátil. No se compartió nada. Si fuiste tú, elige Empezar de nuevo para obtener un código nuevo.
pair-old-locked-hotspot = Si no sabes quién fue, conecta los dos portátiles al punto de acceso de tu teléfono y luego elige Empezar de nuevo.
pair-old-start-again = Empezar de nuevo
pair-old-timed-out = No se eligió ningún número a tiempo, así que aquí tienes un código nuevo.
pair-old-declined = Los números no coincidían, así que no se compartió nada. Aquí tienes un código nuevo.
pair-old-left = El portátil nuevo dejó de vincularse antes de elegir un número, así que no se compartió nada. Aquí tienes un código nuevo.
pair-network-title = Elige tu Wi-Fi.
pair-network-hint = Elige la red en la que está tu portátil nuevo.

## C05 Old laptop: match the number

pair-choose-eyebrow = Confirma la conexión
pair-choose-title = Elige el número que coincide.
pair-choose-body = Mira tu portátil nuevo. Elige el número que muestra.
pair-choose-number = Número { $number }
pair-choose-confirm = Confirmar estos portátiles
pair-choose-renewed = Hay números nuevos. Elige el que aparece en tu portátil nuevo.
pair-choose-none = Ninguno coincide
pair-choose-note = Si el portátil nuevo no muestra ningún número, o ninguno de estos coincide, elige Ninguno coincide.
pair-choose-safe = Todavía no se ha compartido nada.
pair-choose-checking = Comprobando…

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = Portátil nuevo
pair-new-searching-title = Buscando tu portátil antiguo…
pair-new-searching-body = Deja PCTwin abierto en tu portátil antiguo, en la misma Wi-Fi que este.
pair-new-search-failed-title = No se pudo buscar tu portátil antiguo.
pair-new-list-title = ¿Cuál es tu portátil antiguo?
pair-new-list-body = Elige el nombre que muestra la pantalla de tu portátil antiguo.
pair-new-same-label = Otro portátil cercano tiene este nombre. Mira la pantalla del portátil antiguo.
pair-new-needs-update = Este portátil tiene una versión más antigua de PCTwin. Actualiza PCTwin en él y vuelve a buscar.
pair-new-none-title = No aparece tu portátil antiguo.
pair-new-none-body = Comprueba que PCTwin esté abierto en tu portátil antiguo y muestre un código, y que los dos portátiles estén en la misma Wi-Fi. Luego busca de nuevo.
pair-new-search-again = Buscar de nuevo
pair-old-same-role = Otro portátil cercano también está configurado como el portátil antiguo. Uno de los dos debe ser el portátil nuevo.
pair-old-switch = Este es el portátil nuevo
pair-new-none-same-role = ¿El otro portátil también eligió Portátil nuevo? Entonces elige Portátil antiguo en él, o cambia este.
pair-new-switch = Este es el portátil antiguo
pair-new-code-title = Escribe el código.
pair-new-code-body = Escribe el código que aparece en { $label }.
pair-new-code-label = Código de seis cifras
pair-new-code-hint = Léelo en la pantalla de tu portátil antiguo. Cambia cada minuto.
pair-new-connect = Conectar
pair-new-connecting = Conectando…
pair-new-other-laptop = Elegir otro portátil
pair-new-number-eyebrow = Confirma la conexión
pair-new-number-title = Elige este número en tu portátil antiguo.
pair-new-number-tile = Tu número
pair-new-number-label = Tu número es el { $number }
pair-new-number-body = Tu portátil antiguo muestra tres números. Elige allí este.
pair-new-number-waiting = Esperando a tu portátil antiguo…

## Paired (both laptops)

pair-done-eyebrow = Todo listo
pair-done-title = Tus portátiles están vinculados.
pair-done-body = Tus dos portátiles están conectados. Ahora el portátil antiguo busca tus archivos y tú apruebas en el nuevo lo que se mueve. Nada se mueve antes de que lo apruebes.
pair-done-continue = Continuar

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = El código debe tener seis cifras. Mira la pantalla del portátil antiguo y escríbelo de nuevo.
pairing-code-wrong = Ese código no coincide. Mira la pantalla del portátil antiguo e inténtalo de nuevo.
pairing-code-expired = Ese código ha cambiado. Escribe el nuevo que aparece en la pantalla del portátil antiguo.
pairing-busy = El portátil antiguo ya se está vinculando con otro portátil. Espera un momento e inténtalo de nuevo.
pairing-wait = El portátil antiguo está preparando un código nuevo. Espéralo y luego escribe el código nuevo.
pairing-locked = La vinculación se detuvo tras demasiados intentos fallidos. En el portátil antiguo, elige Empezar de nuevo.
pairing-unreachable = Este portátil no pudo llegar al portátil antiguo. Comprueba que los dos estén en la misma Wi-Fi e inténtalo de nuevo.
pairing-wrong-number = Ese número no coincide con el del portátil nuevo. No se compartió nada. Escribe el código nuevo en el portátil nuevo para intentarlo de nuevo.
pairing-no-match = Los números no coincidieron en el portátil antiguo. No se compartió nada. Escribe el código nuevo que aparece en la pantalla del portátil antiguo.
pairing-timed-out = No se eligió ningún número a tiempo. Inténtalo de nuevo con el código nuevo del portátil antiguo.
pairing-connection-lost = Se cortó la conexión entre los portátiles. No se compartió nada. Inténtalo de nuevo.
pairing-connection-lost-new = Se cortó la conexión con el portátil antiguo. No se compartió nada.
pairing-different-version = Los dos portátiles tienen versiones distintas de PCTwin. Actualiza PCTwin en los dos e inténtalo de nuevo.
pairing-name-invalid = Ese nombre no se puede usar. Prueba un nombre más corto, con letras y números.
pairing-no-network = Este portátil no está en una red Wi-Fi ni con cable. Conéctalo e inténtalo de nuevo.
pairing-choose-network = Este portátil está en más de una red. Elige la red de tu portátil nuevo:
pairing-local-network-refused = Puede que tu Mac no deje que PCTwin encuentre portátiles en esta red. Si tu Mac acaba de preguntarte, elige Permitir e inténtalo de nuevo. Si no, abre Ajustes del Sistema, elige Privacidad y seguridad y luego Red local, activa PCTwin e inténtalo de nuevo.
pairing-local-network-refused-details = Una VPN, un cortafuegos o una app de filtrado también pueden impedir que los portátiles se encuentren. Si tienes uno activo, desactívalo mientras haces la vinculación e inténtalo de nuevo.
pairing-failed = La vinculación no funcionó esta vez. No se compartió nada. Inténtalo de nuevo.
pairing-failed-new = El emparejamiento no funcionó esta vez. No se compartió nada. Escribe el código nuevo que aparece en la pantalla del portátil antiguo.

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = Tu traslado
people-title = Asigna a cada persona su cuenta.
people-body = Elige adónde van los archivos de cada persona en el portátil nuevo.
people-col-old = Portátil antiguo
people-col-new = Portátil nuevo
people-col-status = Estado
people-you = Tú
# $name is the person's name on the old laptop. $account is an account name on the new laptop.
people-select =
    .aria-label = Cuenta en el portátil nuevo para { $name }
people-option-existing = { $account } / portátil nuevo
people-option-new = Crear una cuenta nueva
people-option-leave-out = Dejar fuera
people-option-choose = Elegir…
people-new-account-hint = El portátil nuevo pide permiso antes de crear una cuenta.
people-suggested = Sugerida porque los nombres coinciden. Comprueba que sea la misma persona.
# $account is an account name on the new laptop.
people-suggested-use = Usar { $account }
people-status-ready = Confirmado
people-status-waiting = Falta su aprobación
people-status-left-out = Fuera del traslado
people-status-choose = Falta tu elección
people-status-changed = Cambiado por ti
people-waiting-body = { $name } debe entrar en su cuenta en el portátil antiguo y dar su aprobación antes de que sus archivos se trasladen. Hasta entonces, nadie toca sus archivos.
people-just-me = En el portátil antiguo solo se encontró tu cuenta.
people-changed-note = Cambiaste adónde van los archivos de alguien. Revísalo antes de continuar.
people-shared-title = Elementos compartidos
people-shared-body = Carpetas compartidas y documentos públicos que son de todos en el portátil antiguo.
people-shared-select =
    .aria-label = Adónde van los elementos compartidos
people-shared-option-shared = Carpetas compartidas del portátil nuevo
people-shared-option-person = Con los archivos de { $name }
people-shared-option-leave-out = Dejar fuera
people-note = PCTwin nunca cambia la contraseña de nadie ni se salta el acceso de otra persona.
people-list-label = Personas del portátil antiguo
people-todo =
    { $count ->
        [0] Todas las personas que pueden trasladarse ya tienen adónde ir.
        [one] Falta elegir para { $count } persona.
        [many] Falta elegir para { $count } de personas.
       *[other] Falta elegir para { $count } personas.
    }
people-waiting-count =
    { $count ->
        [one] { $count } persona espera para dar su aprobación. Puedes continuar; nadie toca sus archivos hasta que la dé.
        [many] { $count } de personas esperan para dar su aprobación. Puedes continuar; nadie toca sus archivos hasta que la den.
       *[other] { $count } personas esperan para dar su aprobación. Puedes continuar; nadie toca sus archivos hasta que la den.
    }
people-continue = Continuar

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = Tu traslado
before-title = Antes de dejar este portátil.
before-body = Revisa esto mientras todavía tienes tu portátil antiguo.
before-licences-title = Licencias de aplicaciones
before-licences-body = Algunas aplicaciones necesitan que traslades tú su licencia.
before-passwords-title = Recuperar tus cuentas
before-passwords-body = Asegúrate de que todavía puedes entrar en tus cuentas y en los discos bloqueados.
before-codes-title = Ten a mano tus códigos de acceso
before-codes-body = Si una cuenta te envía un código o usa una aplicación para confirmar que eres tú, ten cerca ese teléfono o esa aplicación.
before-online-title = Trabajo sin guardar
before-online-body = Guarda los borradores y comprueba que los archivos guardados en línea estén al día.
before-done =
    .aria-label = Ya lo hice: { $item }
before-note = No escribas aquí contraseñas ni claves de recuperación. Terminar esta lista no significa que ya puedas borrar el portátil antiguo sin riesgo.
before-empty = No hay nada que hacer en tu portátil antiguo antes del traslado.
before-continue = Volver al plan del traslado

## S11 Get help with this task (a short problem code)

help-eyebrow = Ayuda y privacidad
help-title = Consigue ayuda con esta tarea.
help-body = Comparte un código corto que muestra a otra persona qué salió mal. Antes, revisa lo que incluye.
help-making = Creando un código…
help-code-title = Código del problema
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = Código del problema { $code }
help-contains-title = Lo que incluye
help-contains-problem = El tipo de problema y hasta dónde llegó la tarea.
help-contains-step = El paso del traslado en el que ocurrió.
help-contains-versions = La versión de PCTwin y el sistema de este portátil, como Windows o macOS.
help-never-title = Lo que nunca incluye
help-chip-included = Incluido
help-chip-excluded = Nunca incluido
help-never-names = Tu nombre ni el de nadie más.
help-never-files = El contenido de tus archivos ni ninguna contraseña.
help-never-network = Nombres de archivos o carpetas, dónde guardas tus archivos ni nombres de Wi-Fi.
help-note = El código no da a nadie acceso a tus portátiles.
help-copy = Copiar código
help-copied = Código copiado.
help-share = Compartir
help-share-hint = Envíalo por WhatsApp o cualquier aplicación.
help-share-text = Mi código de problema de PCTwin es { $code }.
help-failed = No se pudo crear un código de problema. Inténtalo de nuevo. Si sigue sin funcionar, describe el problema con tus propias palabras.
help-try-again = Intentar de nuevo
help-copy-failed = No se pudo copiar el código. Selecciónalo arriba y cópialo tú.
help-home-eyebrow = Ayuda
help-home-title = Ayuda para lo que estás haciendo.
help-home-body = Guías breves para lo que más a menudo se interpone. Funcionan sin internet.
help-home-more = Más ayuda en getpctwin.com
help-guide-find-title = Los portátiles no se encuentran
help-guide-find-body = Conecta los dos portátiles a la misma Wi-Fi, no a una red de invitados. Desactiva cualquier VPN mientras los emparejas. Si aún no funciona, activa el punto de acceso del móvil y conecta los dos portátiles a él.
help-guide-mac-network-title = Tu Mac pregunta por la red local
help-guide-mac-network-body = Elige Permitir para que PCTwin encuentre tu otro portátil. Si elegiste No permitir, abre Configuración del Sistema, luego Privacidad y seguridad, luego Red local, y activa PCTwin.
help-guide-firewall-title = Windows pregunta por el firewall
help-guide-firewall-body = Permite PCTwin en redes privadas, como la Wi-Fi de casa. PCTwin solo se comunica con tu otro portátil, en tu propia red.
help-guide-stopped-title = Un traslado se detuvo a medias
help-guide-stopped-body = No se pierde nada de lo copiado y comprobado, y tu portátil antiguo no cambió. Vuelve a emparejar los portátiles: los archivos que ya están en el nuevo se reconocen y no se copian dos veces.
help-guide-others-title = Mover las cosas de otras personas
help-guide-others-body = Márcalos en ¿Quién se muda? en el portátil antiguo. Un administrador lo aprueba una vez en cada portátil. Nunca ves sus archivos. La contraseña temporal de una cuenta nueva se muestra una vez en una ventana del propio portátil; la persona elige la suya al iniciar sesión por primera vez.
help-guide-code-title = Pedir ayuda a alguien
help-guide-code-body = En cualquier página de problema, elige Obtener ayuda. PCTwin crea un código corto que dice qué falló, sin nombres, archivos ni contraseñas, para que lo envíes por WhatsApp o cualquier app.

## Label words (the name each laptop shows while pairing)

label-colour-red =
    { $gender ->
        [feminine] roja
       *[masculine] rojo
    }
label-colour-orange = naranja
label-colour-yellow =
    { $gender ->
        [feminine] amarilla
       *[masculine] amarillo
    }
label-colour-green = verde
label-colour-blue = azul
label-colour-purple = violeta
label-colour-pink = rosa
label-colour-brown = marrón
label-colour-black =
    { $gender ->
        [feminine] negra
       *[masculine] negro
    }
label-colour-white =
    { $gender ->
        [feminine] blanca
       *[masculine] blanco
    }
label-colour-grey = gris
label-colour-gold =
    { $gender ->
        [feminine] dorada
       *[masculine] dorado
    }

label-animal-ant = Hormiga
label-animal-badger = Tejón
label-animal-bat = Murciélago
label-animal-bear = Oso
label-animal-bee = Abeja
label-animal-beetle = Escarabajo
label-animal-bison = Bisonte
label-animal-butterfly = Mariposa
label-animal-camel = Camello
label-animal-cat = Gato
label-animal-cheetah = Guepardo
label-animal-chicken = Gallina
label-animal-crab = Cangrejo
label-animal-crane = Grulla
label-animal-crocodile = Cocodrilo
label-animal-crow = Cuervo
label-animal-deer = Ciervo
label-animal-dolphin = Delfín
label-animal-donkey = Burro
label-animal-dove = Paloma
label-animal-dragonfly = Libélula
label-animal-duck = Pato
label-animal-eagle = Águila
label-animal-elephant = Elefante
label-animal-falcon = Halcón
label-animal-flamingo = Flamenco
label-animal-fox = Zorro
label-animal-frog = Rana
label-animal-gazelle = Gacela
label-animal-swordfish = Pez espada
label-animal-giraffe = Jirafa
label-animal-goat = Cabra
label-animal-goose = Ganso
label-animal-gorilla = Gorila
label-animal-hamster = Hámster
label-animal-hawk = Gavilán
label-animal-hedgehog = Erizo
label-animal-armadillo = Armadillo
label-animal-hippo = Hipopótamo
label-animal-horse = Caballo
label-animal-hummingbird = Colibrí
label-animal-jaguar = Jaguar
label-animal-jellyfish = Medusa
label-animal-kangaroo = Canguro
label-animal-kingfisher = Martín pescador
label-animal-koala = Koala
label-animal-ladybird = Mariquita
label-animal-lemur = Lémur
label-animal-leopard = Leopardo
label-animal-lion = León
label-animal-lizard = Lagarto
label-animal-llama = Llama
label-animal-lobster = Langosta
label-animal-lynx = Lince
label-animal-macaw = Guacamayo
label-animal-meerkat = Suricata
label-animal-mole = Topo
label-animal-moose = Alce
label-animal-pufferfish = Pez globo
label-animal-mouse = Ratón
label-animal-octopus = Pulpo
label-animal-ostrich = Avestruz
label-animal-otter = Nutria
label-animal-owl = Búho
label-animal-panda = Panda
label-animal-kiwi = Kiwi
label-animal-parrot = Loro
label-animal-peacock = Pavo real
label-animal-pelican = Pelícano
label-animal-penguin = Pingüino
label-animal-puffin = Frailecillo
label-animal-rabbit = Conejo
label-animal-raccoon = Mapache
label-animal-rhino = Rinoceronte
label-animal-robin = Petirrojo
label-animal-salmon = Salmón
label-animal-seahorse = Caballito de mar
label-animal-seal = Foca
label-animal-shark = Tiburón
label-animal-sheep = Oveja
label-animal-sloth = Perezoso
label-animal-snail = Caracol
label-animal-sparrow = Gorrión
label-animal-squirrel = Ardilla
label-animal-starfish = Estrella de mar
label-animal-stork = Cigüeña
label-animal-swan = Cisne
label-animal-tiger = Tigre
label-animal-snake = Serpiente
label-animal-toucan = Tucán
label-animal-turtle = Tortuga
label-animal-walrus = Morsa
label-animal-whale = Ballena
label-animal-wolf = Lobo
label-animal-woodpecker = Pájaro carpintero
label-animal-yak = Yak
label-animal-zebra = Cebra
label-animal-dog = Perro
label-animal-cow = Vaca
label-animal-beaver = Castor

# Grammatical gender of each animal noun above (value is never shown; see app/l10n/cards/labels.md).
label-gender-ant = feminine
label-gender-badger = masculine
label-gender-bat = masculine
label-gender-bear = masculine
label-gender-bee = feminine
label-gender-beetle = masculine
label-gender-bison = masculine
label-gender-butterfly = feminine
label-gender-camel = masculine
label-gender-cat = masculine
label-gender-cheetah = masculine
label-gender-chicken = feminine
label-gender-crab = masculine
label-gender-crane = feminine
label-gender-crocodile = masculine
label-gender-crow = masculine
label-gender-deer = masculine
label-gender-dolphin = masculine
label-gender-donkey = masculine
label-gender-dove = feminine
label-gender-dragonfly = feminine
label-gender-duck = masculine
label-gender-eagle = feminine
label-gender-elephant = masculine
label-gender-falcon = masculine
label-gender-flamingo = masculine
label-gender-fox = masculine
label-gender-frog = feminine
label-gender-gazelle = feminine
label-gender-swordfish = masculine
label-gender-giraffe = feminine
label-gender-goat = feminine
label-gender-goose = masculine
label-gender-gorilla = masculine
label-gender-hamster = masculine
label-gender-hawk = masculine
label-gender-hedgehog = masculine
label-gender-armadillo = masculine
label-gender-hippo = masculine
label-gender-horse = masculine
label-gender-hummingbird = masculine
label-gender-jaguar = masculine
label-gender-jellyfish = feminine
label-gender-kangaroo = masculine
label-gender-kingfisher = masculine
label-gender-koala = masculine
label-gender-ladybird = feminine
label-gender-lemur = masculine
label-gender-leopard = masculine
label-gender-lion = masculine
label-gender-lizard = masculine
label-gender-llama = feminine
label-gender-lobster = feminine
label-gender-lynx = masculine
label-gender-macaw = masculine
label-gender-meerkat = feminine
label-gender-mole = masculine
label-gender-moose = masculine
label-gender-pufferfish = masculine
label-gender-mouse = masculine
label-gender-octopus = masculine
label-gender-ostrich = masculine
label-gender-otter = feminine
label-gender-owl = masculine
label-gender-panda = masculine
label-gender-kiwi = masculine
label-gender-parrot = masculine
label-gender-peacock = masculine
label-gender-pelican = masculine
label-gender-penguin = masculine
label-gender-puffin = masculine
label-gender-rabbit = masculine
label-gender-raccoon = masculine
label-gender-rhino = masculine
label-gender-robin = masculine
label-gender-salmon = masculine
label-gender-seahorse = masculine
label-gender-seal = feminine
label-gender-shark = masculine
label-gender-sheep = feminine
label-gender-sloth = masculine
label-gender-snail = masculine
label-gender-sparrow = masculine
label-gender-squirrel = feminine
label-gender-starfish = feminine
label-gender-stork = feminine
label-gender-swan = masculine
label-gender-tiger = masculine
label-gender-snake = feminine
label-gender-toucan = masculine
label-gender-turtle = feminine
label-gender-walrus = feminine
label-gender-whale = feminine
label-gender-wolf = masculine
label-gender-woodpecker = masculine
label-gender-yak = masculine
label-gender-zebra = feminine
label-gender-dog = masculine
label-gender-cow = feminine
label-gender-beaver = masculine

# Spanish label order: animal first, then the colour that agrees with it, for example "Zorro azul".
label-picked = { $animal } { $colour }
label-unnamed = Portátil sin nombre

## M02 Finding your files

scan-eyebrow = Tu traslado
scan-title = Buscando tu configuración de siempre.
scan-body = Lo que PCTwin encuentra se guarda, así que la búsqueda puede seguir si se detiene.
scan-progress =
    .aria-label = Buscando en los lugares que elegiste
# $size is passed but not shown in the mockup line.
scan-found =
    { NUMBER($count) ->
        [one] { $count } archivo encontrado hasta ahora.
        [many] { $count } de archivos encontrados hasta ahora.
       *[other] { $count } archivos encontrados hasta ahora.
    }
scan-step-usual-title = Archivos y carpetas
scan-step-usual-body = Se buscó en los lugares que PCTwin puede leer.
scan-step-other-title = Otras carpetas y discos
scan-step-other-body = Carpetas y discos que usas fuera de los lugares habituales.
scan-step-protected-title = Lugares protegidos
scan-step-protected-body =
    { NUMBER($count) ->
        [one] { $count } lugar aún necesita tu permiso.
        [many] { $count } de lugares aún necesitan tu permiso.
       *[other] { $count } lugares aún necesitan tu permiso.
    }
scan-step-done = Hecho
scan-step-current = Ahora
scan-step-todo = Después
scan-unreadable-note = Los lugares que PCTwin no pudo leer siguen en la lista. No cuentan como revisados.
scan-status-not-searched = Sin revisar
scan-paused = Búsqueda en pausa. Todo lo encontrado hasta ahora está guardado.
scan-resumed = La búsqueda siguió donde se había detenido.
scan-partial =
    { NUMBER($count) ->
        [one] Todavía falta buscar en { $count } lugar protegido.
        [many] Todavía falta buscar en { $count } de lugares protegidos.
       *[other] Todavía falta buscar en { $count } lugares protegidos.
    }
scan-allow = Permitir acceso
scan-pause = Pausar
scan-resume = Seguir buscando
scan-continue = Ver lo encontrado

## M03 Everything you chose, in view

overview-eyebrow = Tu traslado
overview-title = Todo lo que elegiste, a la vista.
overview-body = Revisa lo que encontró PCTwin antes de que prepare el plan.
# The metric value next to each label is passed in already formatted.
overview-size-label = archivos seleccionados
overview-items-label =
    { NUMBER($count) ->
        [one] archivo
        [many] de archivos
       *[other] archivos
    }
overview-attention-label =
    { NUMBER($count) ->
        [one] necesita tu atención
        [many] necesitan tu atención
       *[other] necesitan tu atención
    }
overview-files-title = Archivos y carpetas
overview-files-body = Documentos, fotos y otras carpetas que elegiste.
overview-dest-title = Adónde van
overview-dest-body = El lugar en el portátil nuevo para cada carpeta.
overview-people-title = Personas
overview-people-body = Elige la cuenta del portátil nuevo para cada persona.
overview-review = Revisar
overview-partial =
    { NUMBER($count) ->
        [one] Todavía falta buscar en { $count } lugar protegido.
        [many] Todavía falta buscar en { $count } de lugares protegidos.
       *[other] Todavía falta buscar en { $count } lugares protegidos.
    }
overview-allow = Permitir acceso
overview-continue = Continuar

## M04 Keep what matters to you

files-eyebrow = Tu traslado
files-title = Conserva lo que te importa.
files-body = Elige carpetas, otros discos y cualquier otro lugar.
files-search-label = Buscar una carpeta
files-search-placeholder = Buscar carpetas
files-select =
    .aria-label = Trasladar { $name }
files-folder-detail =
    { NUMBER($count) ->
        [one] { $size } · { $count } archivo
        [many] { $size } · { $count } de archivos
       *[other] { $size } · { $count } archivos
    }
files-status-protected = Necesita permiso
files-protected-body = PCTwin necesita tu permiso para leer esta carpeta. Sus archivos no se cuentan hasta que lo permitas.
files-allow = Permitir acceso
files-status-added = Añadida por ti
files-add = Añadir otra carpeta
files-remove =
    .aria-label = Quitar { $name }
files-no-limit = Los traslados gratis no tienen límite de tamaño de archivo ni de número de carpetas.
files-total =
    { NUMBER($count) ->
        [one] { $count } carpeta elegida, { $size } en total
        [many] { $count } de carpetas elegidas, { $size } en total
       *[other] { $count } carpetas elegidas, { $size } en total
    }
files-none = Elige al menos una carpeta para continuar.
files-continue = Elegir destinos

## M05 Give every file a home

dest-eyebrow = Tu traslado
dest-title = Dale un lugar a cada archivo.
dest-body = Elige adónde va cada carpeta en el portátil nuevo.
dest-col-chosen = Lo que elegiste
dest-col-place = Adónde va
dest-col-placement = Disco
dest-placement-local = Interno
dest-placement-external = Externo
dest-col-status = Estado
dest-select =
    .aria-label = Destino de { $name }
# $folder is a folder name on the new laptop, for example "Documents".
dest-option-folder = Portátil nuevo / { $folder }
# $drive is the name of a drive connected to the new laptop.
dest-option-drive = Disco externo / { $drive }
dest-option-choose = Elegir carpeta
dest-default-label = Lugar habitual
dest-status-ready = Listo
dest-status-missing = Falta el disco
dest-status-space = No hay espacio suficiente
dest-all-ready = Cada carpeta tiene un lugar en el portátil nuevo.
dest-missing = { $drive } no está conectado al portátil nuevo. Conéctalo o elige otro lugar para estos archivos.
dest-space = { $place } tiene { $free } libres, pero estos archivos necesitan { $needed }. Libera espacio allí, elige otro lugar o traslada menos archivos.
dest-change = Cambiar
dest-continue = Revisar el plan

## M23 Approve the move you want

plan-eyebrow = Tu traslado
plan-title = Aprueba el traslado que quieres.
plan-body = PCTwin hace solo lo que dice este plan.
plan-what-title = Lo que elegiste
# $size is already formatted, for example "120 GB". $count is passed but not shown.
plan-what-body = { $size } de archivos, de las carpetas que elegiste, a los lugares que indicaste.
plan-where-body =
    { NUMBER($count) ->
        [one] A { $count } lugar del portátil nuevo.
        [many] A { $count } de lugares del portátil nuevo.
       *[other] A { $count } lugares del portátil nuevo.
    }
plan-hand-title = Antes de empezar
plan-hand-body =
    { NUMBER($count) ->
        [one] { $count } paso. Aparece en tu informe al final.
        [many] { $count } de pasos. Aparecen en tu informe al final.
       *[other] { $count } pasos. Aparecen en tu informe al final.
    }
plan-hand-none = Nada, por lo que PCTwin puede ver ahora.
plan-old-safe = Tu portátil antiguo no cambia. Los archivos se copian, nunca se quitan de él.
plan-edit = Cambiar el plan
plan-edit-note = Si cambias el plan, lo vuelves a aprobar antes de que empiece algo nuevo.
plan-approve = Apruebo este plan, incluido lo que debo hacer yo.
plan-not-approved = Todavía no has aprobado este plan, así que no se trasladará nada.
plan-changed = Cambiaste el plan, así que apruébalo de nuevo antes de que se traslade algo nuevo.
plan-start = Empezar el traslado

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = Atrás
# $n is a count of files or folders.
plan-number = { NUMBER($n) }
# $n is a number already rounded for reading, for example 18.4.
plan-size-b = { NUMBER($n) } bytes
plan-size-kb = { NUMBER($n) } KB
plan-size-mb = { NUMBER($n) } MB
plan-size-gb = { NUMBER($n) } GB
plan-size-tb = { NUMBER($n) } TB
# $size is a size already written out, for example "18.4 GB". Used when part of a folder could not be read.
plan-size-at-least = al menos { $size }
plan-size-unknown = tamaño desconocido
plan-role-home = Carpeta personal
plan-role-desktop = Escritorio
plan-role-documents = Documentos
plan-role-downloads = Descargas
plan-role-pictures = Imágenes
plan-role-music = Música
plan-role-videos = Videos
plan-role-public = Carpeta pública
plan-role-app-data = Datos de aplicación
plan-role-other = Otra carpeta
# $done and $total are counts of places this search has looked at and has to look at.
scan-places = { $done } de { $total } lugares revisados
scan-card-title = Buscando en los lugares que elegiste
scan-pausing = Pausando al terminar la carpeta actual. Todo lo encontrado hasta ahora está guardado.
overview-row-attention = Necesita atención
files-status-work = Trabajo o estudios
files-work-body = Tu trabajo o centro de estudios gestiona esta carpeta, así que queda fuera a menos que la marques.
# $count is how many files in the folder are only online.
files-online-only =
    { NUMBER($count) ->
        [one] { $count } archivo está solo en línea, así que no se copia.
        [many] { $count } de archivos están solo en línea, así que no se copian.
       *[other] { $count } archivos están solo en línea, así que no se copian.
    }
files-no-match = Ninguna carpeta tiene ese nombre.
dest-status-none = Elige un lugar
dest-status-readonly = No se puede escribir
dest-status-format = Archivo demasiado grande
# $name is the folder that needs a place.
dest-none = Elige un lugar para { $name } en el portátil nuevo.
# $place is a drive on the new laptop that cannot be written to.
dest-readonly = { $place } es de solo lectura, así que no se puede copiar nada en él. Elige otro lugar.
# $place is the drive. Some drive formats cannot hold a single very large file.
dest-format = Un archivo de estas carpetas es demasiado grande para cómo está preparado { $place }. Elige otro lugar.
# $folder is a folder on the new laptop and $drive the drive it is on.
plan-place = { $folder } en { $drive }
plan-group-to = Va a { $place }
plan-group-move = Mover { $folder }
plan-place-change = Elegir otro lugar
plan-place-usual = Usar el lugar habitual
plan-place-free = { $size } libres ahí
plan-place-no-room = No hay suficiente espacio libre ahí. Elige otro lugar o desmarca esta carpeta.
plan-place-no-safe-name = PCTwin no puede guardar archivos de forma segura en esa unidad. Elige otro lugar.
plan-place-cannot-open = PCTwin no pudo abrir esa carpeta. Elige otro lugar.
plan-place-locked = Esa unidad está bloqueada. Desbloquéala primero y vuelve a elegirla.
plan-version = Versión del plan { $revision }
plan-details = Más detalles
plan-preview-title = Antes de empezar
plan-preview-fits = Hay suficiente espacio libre en este portátil.
plan-preview-fits-about = Debería haber suficiente espacio libre en este portátil. Se cuentan unos { $extra } para las aplicaciones que instala PCTwin y sus datos.
# Added 23 October 2026: what moves first, from the move's own order. $places is a list of folder names.
plan-preview-first = Trasladaremos esto primero, para que tu portátil nuevo esté listo antes: { $places }.
plan-preview-short = Este portátil necesita { $size } más de espacio libre. Libera espacio antes de empezar o el traslado se detendrá cuando el disco se llene.
plan-preview-short-about = Este portátil necesita unos { $size } más de espacio libre, contando unos { $extra } para las aplicaciones que instala PCTwin y sus datos. Libera espacio antes de empezar o el traslado se detendrá cuando el disco se llene.
plan-preview-space-unknown = PCTwin no pudo leer cuánto espacio libre tiene este portátil.
plan-preview-synced = En este portátil, { $folder } se guarda en { $service }. Los archivos que se trasladen ahí se subirán a { $service } y usarán tu espacio allí.
plan-preview-time-one =
    { NUMBER($minutes) ->
        [one] Unos { $minutes } minuto.
        [many] Unos { $minutes } de minutos.
       *[other] Unos { $minutes } minutos.
    }
plan-preview-time = Entre { $low } y { $high } minutos, aproximadamente.
plan-preview-renamed =
    { NUMBER($count) ->
        [one] { $count } archivo tendrá un nombre algo distinto, porque este portátil no admite el suyo. El informe lo indica.
        [many] { $count } de archivos tendrán un nombre algo distinto, porque este portátil no admite los suyos. El informe los indica uno a uno.
       *[other] { $count } archivos tendrán un nombre algo distinto, porque este portátil no admite los suyos. El informe los indica uno a uno.
    }
plan-preview-too-big =
    { NUMBER($count) ->
        [one] { $count } archivo es demasiado grande para el disco de este portátil y no se copiará.
        [many] { $count } de archivos son demasiado grandes para el disco de este portátil y no se copiarán.
       *[other] { $count } archivos son demasiado grandes para el disco de este portátil y no se copiarán.
    }
plan-preview-refused =
    { NUMBER($count) ->
        [one] { $count } archivo tiene un nombre o carpeta que este portátil no admite, y no se copiará.
        [many] { $count } de archivos tienen un nombre o carpeta que este portátil no admite, y no se copiarán.
       *[other] { $count } archivos tienen un nombre o carpeta que este portátil no admite, y no se copiarán.
    }
plan-preview-runs-at-sign-in =
    { NUMBER($count) ->
        [one] { $count } archivo se abriría solo cuando alguien inicie sesión en este portátil. Comprueba que lo esperas.
        [many] { $count } de archivos se abrirían solos cuando alguien inicie sesión en este portátil. Comprueba que los esperas.
       *[other] { $count } archivos se abrirían solos cuando alguien inicie sesión en este portátil. Comprueba que los esperas.
    }
# $reference is a short code that names exactly this version of the plan.
plan-reference = Referencia { $reference }
# $count is how many files are affected in each line below.
plan-hand-cloud-only =
    { NUMBER($count) ->
        [one] { $count } archivo está solo en línea. PCTwin no lo descarga. Haz que esté disponible antes en el portátil antiguo, o déjalo.
        [many] { $count } de archivos están solo en línea. PCTwin no los descarga. Haz que estén disponibles antes en el portátil antiguo, o déjalos.
       *[other] { $count } archivos están solo en línea. PCTwin no los descarga. Haz que estén disponibles antes en el portátil antiguo, o déjalos.
    }
plan-hand-unreadable =
    { NUMBER($count) ->
        [one] { $count } elemento no se pudo abrir, así que no se traslada. Permite el acceso, o déjalo.
        [many] { $count } de elementos no se pudieron abrir, así que no se trasladan. Permite el acceso, o déjalos.
       *[other] { $count } elementos no se pudieron abrir, así que no se trasladan. Permite el acceso, o déjalos.
    }
plan-hand-work-managed =
    { NUMBER($count) ->
        [one] { $count } elemento lo gestiona tu trabajo o centro de estudios y queda fuera. Marca su carpeta para incluirlo.
        [many] { $count } de elementos los gestiona tu trabajo o centro de estudios y quedan fuera. Marca sus carpetas para incluirlos.
       *[other] { $count } elementos los gestiona tu trabajo o centro de estudios y quedan fuera. Marca sus carpetas para incluirlos.
    }
plan-hand-too-many-files =
    { NUMBER($count) ->
        [one] { $count } carpeta tiene más archivos de los que cabe en una sola mudanza, así que no se mueve. Después de esta mudanza, envíala por separado con Envío rápido o cópiala con una memoria USB.
        [many] { $count } de carpetas tienen más archivos de los que caben en una sola mudanza, así que no se mueven. Después de esta mudanza, envía cada una por separado con Envío rápido o cópialas con una memoria USB.
       *[other] { $count } carpetas tienen más archivos de los que caben en una sola mudanza, así que no se mueven. Después de esta mudanza, envía cada una por separado con Envío rápido o cópialas con una memoria USB.
    }
plan-blocked-someone-unmapped = Alguien del portátil antiguo aún no tiene cuenta elegida en el nuevo. Elige adónde van los archivos de cada persona y vuelve.
plan-blocked-account-used-twice = Dos personas acabarían en la misma cuenta del portátil nuevo. Dale a cada persona la suya.
plan-blocked-invalid = Este plan no se puede aprobar tal como está. Vuelve atrás y revisa lo que elegiste.
plan-blocked-fix = Elegir cuentas

## Tray menu

tray-show = Mostrar PCTwin
tray-quit = Salir de PCTwin

## F01 secondary actions and their coming-soon pages

welcome-restore = Restaurar configuración
welcome-rescue = Rescatar un portátil
welcome-quick-send = Envío rápido

soon-eyebrow = Muy pronto
soon-restore-title = Restaurar configuración
soon-restore-body = Rehaz un portátil a partir de la configuración que guardaste tras un traslado. Llegará en una versión posterior.
soon-rescue-title = Rescatar un portátil
soon-rescue-body = Recupera los archivos de un portátil que no arranca conectando su disco a uno que funcione. Llegará en una versión posterior.
soon-move-title = Traslado completo
soon-move-body = Pasa todo de tu portátil antiguo a uno nuevo: tus archivos, aplicaciones y ajustes. Llegará en una versión posterior. Mientras tanto, usa el Envío rápido para enviar archivos.

## X01 Move progress, compact

progress-eyebrow = Tu traslado
progress-title = Preparando tu nuevo portátil.
progress-title-old = Enviando tus cosas a tu nuevo portátil.
progress-title-paused = Traslado en pausa
progress-paired = Vinculados
progress-percent =
    .aria-label = Progreso del traslado
# $done and $total are already formatted sizes, for example "81.6 GB" and "120 GB".
progress-amount = { $done } de { $total }
# $range is a measured range, already formatted, for example "15 to 20 minutes".
progress-time-left = Faltan unos { $range }
progress-time-measuring = Calculando el tiempo que falta…
# $speed is measured and already formatted, for example "62 MB/s".
progress-speed = { $speed }, medida
progress-count-needs-action =
    { NUMBER($count) ->
        [one] { $count } elemento necesita que hagas algo
        [many] { $count } de elementos necesitan que hagas algo
       *[other] { $count } elementos necesitan que hagas algo
    }
progress-current-copying = Copiando: { $name }
progress-current-checking = Comprobando: { $name }
progress-needs-action =
    { NUMBER($count) ->
        [one] { $count } elemento necesita que hagas algo. El resto sigue trasladándose.
        [many] { $count } de elementos necesitan que hagas algo. El resto sigue trasladándose.
       *[other] { $count } elementos necesitan que hagas algo. El resto sigue trasladándose.
    }
progress-see-needs-action = Ver lo que necesita tu ayuda
progress-interrupted = Se cortó la conexión con el otro portátil. PCTwin está intentando reconectar. No se pierde nada de lo copiado y comprobado.
progress-low-battery = A este portátil le queda poca batería. Conéctalo para que el traslado no se interrumpa.
progress-battery-paused = La batería de este portátil está casi vacía, así que el traslado se ha pausado con seguridad. Conéctalo y luego continúa. No se pierde nada.
progress-old-safe = Tu portátil antiguo no cambia.
progress-pause = Pausar el traslado
progress-compact = Vista reducida
progress-expand = Ver todo el traslado
progress-stop = Parar el traslado

## X02 Move progress, expanded

progress-all-title = Todo el traslado.
progress-all-body = Cada elemento muestra de dónde viene y cómo va.
progress-metric-overall = progreso total
progress-metric-committed = copiado y comprobado
progress-metric-estimate = estimación
progress-col-item = Elemento
progress-col-from = Origen
progress-col-status = Estado
progress-state-waiting = En espera
progress-state-copying = Copiando
progress-state-verified = Comprobado
progress-state-checking = Comprobando
progress-state-paused = En pausa
progress-checked-note = Un elemento solo cuenta como copiado después de comprobarlo.
# Added 23 October 2026: T14, one tip under the progress bar, about every 12 seconds.
tips-label = Consejo
tips-pause = Pausar los consejos
tips-resume = Ver más consejos
tips-next = Siguiente consejo
tip-battery = Este portátil funciona con batería. Enchúfalo si puedes, para que un traslado largo no se corte.
tip-closer = La conexión es lenta. Acercar los portátiles al router Wi-Fi puede acelerarla.
tip-lids = Deja abierta la tapa de este portátil. Una tapa cerrada puede dejarlo en reposo, y eso pausa el traslado.
tip-keep-using = Puedes seguir usando este portátil. PCTwin solo lee de él.
tip-pause = Puedes pausar el traslado cuando quieras y seguir más tarde.
tip-power-cut = Si se va la luz, enciende los dos portátiles y abre PCTwin. Sigue desde donde se quedó, y no se pierde nada de lo copiado y comprobado.

## F09 Mini panel

mini-title = Tu traslado, aquí cerca.
mini-title-paused = Traslado en pausa.
mini-what = trasladando los archivos seleccionados
mini-progress =
    .aria-label = Progreso del traslado
mini-pause = Pausar
mini-resume = Reanudar
mini-open = Abrir traslado
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = Tus archivos están en camino.
mini-what-send = enviando los archivos seleccionados
mini-open-send = Abrir envío
mini-progress-send =
    .aria-label = Progreso del envío

## X05 Paused at a safe point

pause-title = En pausa en un punto seguro.
pause-title-finishing = Pausando en un punto seguro…
pause-body = Se conserva todo lo copiado y comprobado. El resto espera.
pause-finishing = Primero se termina { $name }, para que no quede a medio copiar.
pause-kept-title = Archivos
pause-kept-body = Se conserva todo lo copiado y comprobado hasta ahora.
pause-waiting-title = En espera
pause-waiting-body =
    { NUMBER($count) ->
        [one] Falta copiar { $count } elemento. No se pierde nada.
        [many] Falta copiar { $count } de elementos. No se pierde nada.
       *[other] Falta copiar { $count } elementos. No se pierde nada.
    }
pause-status-kept = En pausa
pause-status-waiting = En espera
pause-note = Pausar no deshace nada. Tu portátil antiguo no cambia.
pause-resume = Reanudar el traslado guardado

## X10 Stop the remaining move? (danger dialog)

cancel-title = ¿Parar lo que falta del traslado?
cancel-body = Al parar se conserva todo lo copiado y comprobado. No deshace el traslado.
cancel-copied-title = Archivos copiados
cancel-copied-body = Se quedan en el portátil nuevo.
cancel-rest-title = El resto del traslado
cancel-rest-body = No empieza nada nuevo. Los archivos aún sin copiar siguen solo en el portátil antiguo.
cancel-partial-title = Un archivo copiado a medias
cancel-partial-body = Nunca se conserva como si estuviera completo.
cancel-old-title = Portátil antiguo
cancel-old-body = No cambia.
cancel-ack = Entiendo que parar no deshace lo que ya se copió
cancel-stop = Parar y ver el informe
cancel-keep = Seguir trasladando

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = Resultado del traslado
outcome-title = Te damos la bienvenida a tu nuevo portátil.
outcome-body = Esto es lo que pasó con todo lo que aprobaste.
outcome-title-old = Tus cosas ya están en tu nuevo portátil.
outcome-body-old = Esto es lo que pasó con todo lo que enviaste. Este portátil sigue como estaba.
outcome-files-title = Archivos seleccionados
outcome-status-partial = Hecho en parte
# Added 23 October 2026: the after-move check on R01, counted the way the scan counts.
outcome-kinds-title = Comprobado después del traslado
outcome-kind-photos = Fotos
outcome-kind-videos = Vídeos
outcome-kind-music = Música
outcome-kind-documents = Documentos
outcome-kind-count = { $copied } de { $planned } copiados y comprobados
outcome-next-title = Qué hacer ahora
outcome-old-safe = Lo que no terminó sigue en esta lista. No se quitó nada de tu portátil antiguo.
outcome-landed-title = Dónde están ahora tus cosas
outcome-report = Ver el informe completo
outcome-home = Volver al inicio
outcome-person-copied =
    { NUMBER($count) ->
        [one] { $count } archivo copiado.
        [many] { $count } de archivos copiados.
       *[other] { $count } archivos copiados.
    }
outcome-person-not-done =
    { NUMBER($count) ->
        [one] { $count } no terminó.
        [many] { $count } de ellos no terminaron.
       *[other] { $count } no terminaron.
    }
outcome-person-at-sign-in = Estarán en su sitio cuando { $name } inicie sesión en este portátil la próxima vez.
move-fallback-folder = De tu portátil antiguo
move-stop = Detener el traslado
move-before-open = Mientras esperas: antes de dejar este portátil
move-get-help = Obtener ayuda
pickup-eyebrow = Tu traslado
pickup-title = Retoma con seguridad.
pickup-body = Un traslado en este portátil se detuvo a medias, quizá porque se reinició o se quedó sin energía. Primero ordena lo pendiente y luego vuelve a emparejar los portátiles para seguir.
pickup-unfinished-title = Copias sin terminar
pickup-unfinished-body =
    { NUMBER($count) ->
        [one] Se estaba copiando { $count } archivo cuando el traslado se detuvo.
        [many] Se estaban copiando { $count } de archivos cuando el traslado se detuvo.
       *[other] Se estaban copiando { $count } archivos cuando el traslado se detuvo.
    }
pickup-unfinished-badge = Por ordenar
pickup-tidy = Ordenar
pickup-tidying = Ordenando. Las copias terminadas se comprueban y se colocan en su sitio; no se cambia nada tuyo.
pickup-done = Todo en orden. Vuelve a emparejar los portátiles para seguir: lo que ya está aquí se reconoce y no se copia dos veces.
pickup-failed = Ordenar no funcionó esta vez. No se cambió nada tuyo. Inténtalo de nuevo, o primero cierra PCTwin en este portátil y vuelve a abrirlo.
pickup-put-in-place = colocados
pickup-kept = guardados para seguir
pickup-not-finished =
    { NUMBER($count) ->
        [one] No se pudo terminar { $count } archivo. El próximo traslado lo vuelve a copiar.
        [many] No se pudieron terminar { $count } de archivos. El próximo traslado los vuelve a copiar.
       *[other] No se pudieron terminar { $count } archivos. El próximo traslado los vuelve a copiar.
    }
pickup-left-for-later =
    { NUMBER($count) ->
        [one] { $count } archivo está en un disco que ahora no está conectado. Conéctalo y vuelve a ordenar.
        [many] { $count } de archivos están en un disco que ahora no está conectado. Conéctalo y vuelve a ordenar.
       *[other] { $count } archivos están en un disco que ahora no está conectado. Conéctalo y vuelve a ordenar.
    }
pickup-later = Ahora no
ready-eyebrow = Hola
ready-title = Tus cosas están listas.
ready-body = Tus cosas del portátil antiguo se trasladaron a este y ya están en tus propias carpetas.
ready-landed = archivos en tus carpetas
ready-where-title = Dónde están
ready-where-body = En Documentos, Imágenes, Música, Vídeos y tus otras carpetas habituales, como en tu portátil antiguo.
ready-kept =
    { NUMBER($count) ->
        [one] { $count } archivo se guardó aparte porque ya había uno con ese nombre. Se conservan los dos.
        [many] { $count } de archivos se guardaron aparte porque ya había otros con esos nombres. Se conservan todos.
       *[other] { $count } archivos se guardaron aparte porque ya había otros con esos nombres. Se conservan todos.
    }
ready-failed =
    { NUMBER($count) ->
        [one] No se pudo poner { $count } archivo en tus carpetas. Pide a quien hizo el traslado que lo vuelva a mover.
        [many] No se pudieron poner { $count } de archivos en tus carpetas. Pide a quien hizo el traslado que los vuelva a mover.
       *[other] No se pudieron poner { $count } archivos en tus carpetas. Pide a quien hizo el traslado que los vuelva a mover.
    }
ready-done = Listo
move-status-kept = Guardado
move-wait-eyebrow = Tu traslado
move-wait-scanning-title = Buscando tus archivos.
move-wait-scanning-body = Tu portátil antiguo está revisando tus carpetas. Solo las lee: no se abre, cambia ni borra nada.
move-found = Encontrado hasta ahora: { $size } de tus cosas.
move-wait-approval-title = Esperando a tu portátil nuevo.
move-wait-approval-body = Tus archivos están listos. Revisa el plan en el portátil nuevo y apruébalo para empezar.
move-wait-list-title = Esperando a tu portátil antiguo.
move-wait-list-body = Tu portátil antiguo está buscando tus cosas y luego allí se elige qué mover. Cuando termine, lo que llega aparecerá aquí para que lo aceptes. Puede tardar unos minutos.
move-wait-starting-title = Empezando el traslado.
move-wait-starting-body = El plan está aprobado. La copia empieza en un momento.
move-wait-admin-title = Esperando a un administrador.
move-wait-admin-body = Este portátil está pidiendo la aprobación de un administrador para mover también las cosas de las demás personas. Responde en la ventana que abrió el portátil.
move-others-not-approved = El administrador no lo aprobó, así que esta vez solo se mueven tus cosas. Las de las demás personas se quedan donde están.
move-others-failed = PCTwin no pudo iniciar lo que necesita para mover las cosas de las demás personas, así que esta vez solo se mueven las tuyas. No se cambió nada suyo.
move-others-not-here = Mover las cosas de otras personas desde este portátil aún no está listo en esta versión, así que solo se mueven las tuyas.
move-others-allow-in-settings = Primero, un administrador debe permitir PCTwin en la página de Ajustes del Sistema que se acaba de abrir. Esta vez solo se mueven tus cosas; cuando PCTwin esté permitido, tu próximo traslado podrá llevar también las cosas de las demás personas.
approve-prompt = PCTwin leerá los archivos de { $names } para moverlos al nuevo portátil. Un administrador de este portátil lo aprueba solo para este traslado.
who-here-title = ¿Quién se muda?
who-here-body = Tus cosas siempre se mueven. Marca a cualquier otra persona de este portátil cuyas cosas también deban moverse. PCTwin nunca te muestra sus archivos.
who-here-you = { $name } (tú)
who-here-you-hint = Siempre se mueve
who-here-approval = Mover las cosas de otra persona necesita la aprobación de un administrador, una sola vez, para todas las personas que marques. El portátil la pedirá en su propia ventana.
who-here-not-here = Mover las cosas de otras personas desde este portátil aún no está listo en esta versión. Solo se mueven las tuyas.
place-title = ¿Adónde van sus cosas?
place-body = También llegaron cosas de otras personas desde el portátil antiguo. Elige para cada persona una cuenta de este portátil, una cuenta nueva o déjala fuera. Nunca verás sus archivos.
place-size =
    { NUMBER($count) ->
        [one] { $count } archivo, { $size }
        [many] { $count } de archivos, { $size }
       *[other] { $count } archivos, { $size }
    }
place-select = Adónde van las cosas de { $name }
place-note = Las cuentas nuevas, y guardar sus cosas hasta que inicien sesión, necesitan la aprobación de un administrador una sola vez, después de que apruebes el plan. La contraseña temporal de cada cuenta nueva se muestra una vez, en una ventana del propio portátil.
place-todo =
    { NUMBER($count) ->
        [one] Elige adónde van las cosas de { $count } persona más.
        [many] Elige adónde van las cosas de { $count } de personas más.
       *[other] Elige adónde van las cosas de { $count } personas más.
    }
fda-title = Tu Mac mantiene algunas carpetas privadas.
fda-body =
    { NUMBER($count) ->
        [one] No se pudo mirar en { $count } de tus lugares por la configuración de privacidad del Mac. Deja entrar a PCTwin y vuelve a comprobar, para que no falte nada tuyo.
        [many] No se pudo mirar en { $count } de tus lugares por la configuración de privacidad del Mac. Deja entrar a PCTwin y vuelve a comprobar, para que no falte nada tuyo.
       *[other] No se pudo mirar en { $count } de tus lugares por la configuración de privacidad del Mac. Deja entrar a PCTwin y vuelve a comprobar, para que no falte nada tuyo.
    }
fda-step-1 = Abre Configuración del Sistema y luego Privacidad y seguridad.
fda-step-2 = Elige Acceso total al disco.
fda-step-3 = Activa PCTwin. Es posible que el Mac pida tu contraseña.
fda-open = Abrir esta configuración
fda-note = PCTwin solo lee tus carpetas para copiarlas. Nunca cambia nada en este portátil.
fda-again = Volver a comprobar
fda-without = Seguir sin ellas
password-title = PCTwin: una cuenta nueva
password-body =
    { $name } ya tiene una cuenta en este portátil.
    
    Nombre de inicio de sesión: { $account }
    Contraseña temporal: { $password }
    
    Dáselos a { $name }. La primera vez que inicie sesión elegirá su propia contraseña. Anótala ahora: PCTwin no la guarda.
move-ended-eyebrow = Tu traslado
move-ended-kept-title = Ya copiado
move-ended-kept-body = Todo lo copiado y comprobado antes de esto se queda en el portátil nuevo.
move-ended-old-title = Tu portátil antiguo
move-ended-old-body = No se cambió ni se borró nada en él.
move-ended-old-badge = Sin cambios
move-ended-next = Para intentarlo de nuevo, vuelve a emparejar los portátiles. Los archivos ya copiados y comprobados se reconocen y no se copian dos veces.
move-lost-title = Se perdió la conexión con tu otro portátil.
move-lost-body = Los portátiles dejaron de comunicarse, así que el traslado no pudo seguir. Comprueba que los dos estén encendidos, activos y cerca.
move-not-approved-title = El traslado no empezó.
move-not-approved-old = El plan no se aprobó en el portátil nuevo, así que no se copió nada.
move-not-approved-new = No aprobaste el plan, así que no se copió nada.
move-failed-title = Algo salió mal.
move-failed-body = El traslado no pudo seguir. Si vuelve a pasar, Más detalles, abajo, explica qué falló.
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = Otro sistema
move-update-here-title = Actualiza PCTwin en este portátil
move-update-here-body = Este portátil tiene una versión de PCTwin más antigua que el otro, así que no pueden mover archivos juntos. Actualiza PCTwin aquí y vuelve a empezar. No se copió nada.
move-update-there-title = Actualiza PCTwin en el otro portátil
move-update-there-body = El otro portátil tiene una versión de PCTwin más antigua que este, así que no pueden mover archivos juntos. Actualiza PCTwin allí y vuelve a empezar. No se copió nada.
move-refused-title = El traslado no pudo empezar.
move-refused-not-paired = Tus portátiles ya no están emparejados. Vuelve a emparejarlos para empezar el traslado.
move-refused-busy = Ya hay un traslado en curso en este portátil.
move-refused-not-moving = No hay ningún traslado en curso en este portátil.
move-refused-plan-changed = El plan cambió antes de aprobarse. Vuelve a emparejar los portátiles para ver el plan nuevo.
move-refused-failed = No se pudieron preparar las carpetas de este portátil para el traslado. Vuelve a emparejar los portátiles para intentarlo otra vez.
drive-problem-title = El traslado necesita tu atención.
drive-problem-body = Las copias guardadas en { $place } no coincidieron con los originales al comprobarlas PCTwin, varias veces. El disco puede estar fallando, lleno o desconectado, así que la copia allí está en pausa.
drive-problem-kept-title = Ya copiado y comprobado
drive-problem-waiting-title = Pendiente de copiar en { $place }
drive-problem-badge = En pausa
drive-problem-fix = Comprueba que el disco esté conectado y tenga espacio libre, y luego continúa. No se deja fuera nada de lo que elegiste para evitarlo. También puedes detener el traslado.
drive-problem-carry-on = Continuar

## R02 Your move, item by item (detailed report)

report-title = Tu traslado, por tipo.
report-body = Abre un tipo para ver sus archivos. Lo que necesita tu atención aparece primero.
report-needs-title = { $count ->
        [one] 1 cosa necesita tu atención
        [many] { $count } cosas necesitan tu atención
       *[other] { $count } cosas necesitan tu atención
    }
report-group-photos = Fotos
report-group-videos = Vídeos
report-group-music = Música
report-group-documents = Documentos
report-group-other = Otros archivos
report-group-size = { $files ->
        [one] 1 archivo
        [many] { $files } archivos
       *[other] { $files } archivos
    } · { $size }
report-group-done = Todo copiado y comprobado
report-group-some = { $done } de { $total } copiados y comprobados
report-group-left = { $count ->
        [one] 1 se quedó en tu portátil antiguo, como elegiste
        [many] { $count } se quedaron en tu portátil antiguo, como elegiste
       *[other] { $count } se quedaron en tu portátil antiguo, como elegiste
    }
report-filter-label = Mostrar
report-col-item = Elemento
report-col-result = Resultado
report-col-details = Comprobación o siguiente paso
report-from = De: { $path }
report-to = A: { $path }
report-none = Ningún elemento tiene este resultado.
report-more-details = Más detalles
# Added 23 October 2026: what the report says beside a line's result (changed since the scan).
report-note-changed-since-scan = Cambió después del análisis: se copió tal como está ahora.
report-note-no-longer-there = Ya no está: se movió, se cambió de nombre o se eliminó en el portátil antiguo antes de poder enviarse.
report-note-time-not-kept = Fecha no conservada: esta unidad no puede guardar la fecha en que se modificó por última vez, así que muestra la fecha en que se copió.

# Evidence: how a copied item was checked.
evidence-read-back = Comprobado volviendo a leerlo del disco.
evidence-memory = Comprobado con la memoria de este portátil, no con el disco.
evidence-already-there = Ya estaba en el portátil nuevo: el mismo archivo ya estaba allí, así que no se volvió a copiar.
evidence-recovered = Comprobado antes de que el portátil se reiniciara, y otra vez después.
evidence-new-laptop = Comprobado por el portátil nuevo.
evidence-memory-explain = La copia se volvió a leer y coincidía, pero a través de la memoria de este portátil, así que lo leído puede venir de la memoria y no del disco. Un fallo del propio disco podría no verse aquí, así que abrir el archivo una vez es una buena comprobación.
evidence-not-checked = Sin comprobar, porque no terminó.
# Machine-translated 16 October 2026.
why-memory-cannot-skip-cache = Este disco no dejó que PCTwin lo leyera directamente, así que la comprobación pasó por la memoria de este portátil. Algunos discos de red y memorias USB funcionan así.
why-memory-not-shown = Este disco no pudo mostrar que la lectura de PCTwin llegó de verdad a él, así que la comprobación cuenta como hecha desde la memoria de este portátil.
why-memory-no-way = El sistema de este portátil no tiene forma de leer este disco directamente, así que la comprobación pasó por su memoria.
why-memory-stored-specially = Este archivo está guardado de una forma especial, por ejemplo comprimido o cifrado, así que el sistema lo lee a través de la memoria.
why-memory-too-small = Este archivo es muy pequeño, así que el disco lo guarda junto con sus propios registros y el sistema lo lee a través de la memoria.

# Reasons: why an item did not finish. Each says what happened, then what to do.
reason-not-enough-space = El portátil nuevo se quedó sin espacio. Libera espacio en él y empieza un traslado nuevo que incluya este elemento.
reason-changed-during-move = Este archivo no dejó de cambiar en el portátil antiguo mientras se copiaba. Cierra la aplicación que lo use y empieza un traslado nuevo que lo incluya.
reason-unreadable = El portátil antiguo no dejó que PCTwin leyera este elemento. Comprueba que se abre en el portátil antiguo y empieza un traslado nuevo que lo incluya.
reason-not-verified = La copia no coincidía con el original, así que no se conservó. Empieza un traslado nuevo que lo incluya. Si vuelve a fallar, puede que el original esté dañado.
reason-refused-by-safety-check = El control de seguridad del portátil nuevo rechazó este elemento por su nombre, su lugar o su tamaño. El original sigue en el portátil antiguo, así que puedes copiarlo de otra forma si lo necesitas.
reason-name-not-allowed = Este nombre no se puede usar en el portátil nuevo, y no fue posible darle otro nombre seguro. Cámbiale el nombre en el portátil antiguo y empieza un traslado nuevo que lo incluya.
reason-install-failed = La aplicación no se instaló. Instálala desde la web de su fabricante.
reason-no-equivalent = No hay versión de esta aplicación para el sistema del portátil nuevo. Busca una aplicación parecida que funcione allí.
reason-drive-locked = La unidad de destino está bloqueada. Desbloquéala (por ejemplo, con su contraseña o clave de recuperación de BitLocker) y vuelve a trasladarlo.
reason-sign-in = Ya está en su sitio, pero tienes que entrar con tu cuenta. Ábrela y entra con tu cuenta.
reason-activation = La aplicación ya está en su sitio, pero necesita su licencia en el portátil nuevo. Ábrela y actívala con tu licencia o tu cuenta.
reason-permission = Primero el portátil nuevo tiene que dar permiso. Abre el elemento y elige Permitir cuando te lo pida.
reason-online-only = Este archivo solo está guardado en línea, así que no estaba en el portátil antiguo para copiarlo. Descárgalo en el portátil nuevo desde la aplicación que lo guarda en línea.
reason-manual-install = Esta aplicación hay que conseguirla en la web de su fabricante. Descárgala e instálala desde allí.
reason-restart = El portátil nuevo tiene que reiniciarse para terminar. Reinícialo cuando te venga bien.
reason-finish-by-hand = PCTwin no puede hacer este paso por ti. Sigue los pasos que aparecen para él en este informe.
reason-parts-missing = Algunos archivos de esta carpeta no llegaron, así que no está completa en el portátil nuevo. Aparecen en este informe con el motivo. Envíalos de nuevo o copia la carpeta de otra forma.
reason-moved-file-missing = Este ajuste apunta a un archivo o una carpeta que no está en el portátil nuevo con el nombre que le dio el traslado, así que PCTwin dejó el ajuste como estaba. Abre la aplicación y vuelve a elegir el archivo.

## Item results (every item ends with one of these)

status-copied = Copiado
status-reinstalled = Reinstalado
status-converted = Convertido
status-needs-action = Requiere acción
status-skipped-by-user = Omitido por ti
status-failed = Falló

## Screens: words the copy deck did not cover (same voice)

# $low and $high are whole minutes, a measured range. The result fills "About { $range } left".
progress-minutes = { $low } a { $high } minutos
# Machine-translated 16 October 2026.
progress-minutes-one =
    { NUMBER($minutes) ->
        [one] { $minutes } minuto
        [many] { $minutes } de minutos
       *[other] { $minutes } minutos
    }
# $low and $high are numbers and $unit is already formatted, for example "MB/s". The result fills "{ $speed }, measured".
progress-speed-range = { $low } a { $high } { $unit }
progress-count-failed =
    { NUMBER($count) ->
        [one] { $count } elemento no se pudo copiar
        [many] { $count } de elementos no se pudieron copiar
       *[other] { $count } elementos no se pudieron copiar
    }
progress-queue-empty = Todavía no ha empezado nada.
progress-approved-job = Traslado aprobado
progress-route-old = Portátil antiguo
progress-route-new = Portátil nuevo
progress-panel-pausing = Pausando
outcome-metric-size = copiado y comprobado
outcome-metric-action = necesitan que hagas algo
progress-percent-unknown = PCTwin todavía está calculando el tamaño de este traslado.
mini-title-send-paused = Envío en pausa.
cancel-ack-hint = Marca la casilla de arriba para activar el botón de parar.
progress-state-placing = Colocando
progress-state-existing = Ya estaba
outcome-stopped-title = Tu traslado se paró.
outcome-stopped-body = Paraste el traslado. Se conserva lo que se copió y comprobó hasta entonces. El resto no se copió.
outcome-broke-title = Tu traslado terminó antes de tiempo.
outcome-broke-body = Algo falló entre los portátiles, así que el traslado no pudo continuar. Se conserva lo que se copió y comprobó hasta entonces. El resto no se copió.
# $done and $total are counts of items: what was copied and checked, and everything the person approved.
outcome-files-summary = { $done } de { $total } elementos copiados y comprobados.
outcome-files-memory =
    { NUMBER($count) ->
        [one] { $count } comprobado con la memoria de este portátil, no con el disco.
        [many] { $count } de elementos comprobados con la memoria de este portátil, no con el disco.
       *[other] { $count } comprobados con la memoria de este portátil, no con el disco.
    }
report-show-more = Mostrar más
# $shown and $total are counts of items.
report-showing = Se muestran { $shown } de { $total }

## Component kit

dialog-close = Cerrar
toast-dismiss = Descartar
code-label = Código de seis cifras

## Shared Quick Send words

qs-eyebrow = Envío rápido
qs-eyebrow-receiving = Recibiendo en tu portátil
qs-label-this = Este portátil
qs-label-other = El otro portátil
qs-pick-failed = No se pudo abrir el selector. Inténtalo de nuevo.
qs-pick-too-many = Son más archivos de los que caben en un solo envío. Desmarca algunas carpetas o envíalas por partes.
qs-problem-lost-title = Se perdió la conexión entre los portátiles.
qs-problem-failed-title = Esta vez no se pudieron enviar los archivos.
qs-problem-body = Lo que ya se recibió y comprobó se queda donde está. Vuelve a emparejar los portátiles para intentarlo otra vez.
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = De { $from } a { $to }
qs-route-sender = Enviando
qs-route-receiver = Recibe · misma Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } elemento seleccionado · { $size }
        [many] { $count } de elementos seleccionados · { $size }
       *[other] { $count } elementos seleccionados · { $size }
    }
qs-tally-badge = Envío cercano

## Q01 Quick Send home

qs-home-title = Envía archivos a un portátil cercano.
qs-home-body = O recibe archivos de uno.
qs-send-title = Enviar archivos
qs-send-body = Elige archivos o carpetas de tu portátil.
qs-receive-title = Recibir archivos
qs-receive-body = Deja que un portátil cercano envíe archivos al tuyo.
qs-home-action = Elegir archivos
qs-receiving = Listo para recibir. Los portátiles cercanos ven el tuyo como { $label }.
qs-receive-stop = Dejar de recibir
qs-no-nearby = No se encontró ningún portátil cerca. Abre PCTwin en el portátil que envía y deja los dos en la misma Wi-Fi.

## Q02 Choose files and folders

qs-choose-title = ¿Qué vas a enviar?
qs-choose-body = Elige solo los archivos y carpetas que quieres compartir.
qs-drop =
    .aria-label = Zona para soltar archivos y carpetas
qs-drop-title = Suelta aquí archivos o carpetas
qs-drop-body = O añádelos con Añadir archivos o Añadir carpetas.
qs-drop-active = Suelta para añadirlos.
qs-add-files = Añadir archivos
qs-add-folders = Añadir carpetas
qs-remove =
    .aria-label = Quitar { $name }
# $kind is the file type, for example "PDF document" or "8 photos".
qs-file-detail = { $size } · { $kind }
qs-empty = Aún no has elegido nada. Añade archivos o carpetas para enviar.
qs-online-only = Algunos archivos solo están en línea. Haz que estén disponibles en tu portátil primero y luego envíalos.
qs-protected = PCTwin no puede leer { $name }. Permite el acceso o quítalo de la lista.
qs-allow = Permitir acceso
qs-choose-continue = Continuar

## Q12 Send progress

qs-sending-title = Tus archivos están en camino.
qs-sending-body = Las copias van directamente al portátil que recibe.
qs-sending-progress =
    .aria-label = Enviando archivos
qs-sending-current = Enviando: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, medida
qs-step-copying = Copiando
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } archivo enviado.
        [many] { $done } de archivos enviados.
       *[other] { $done } archivos enviados.
    }
qs-step-checking = Comprobando
qs-step-checking-body = Cada archivo se compara con el original.
qs-sending-paused = En pausa. Se conservan los archivos ya recibidos y comprobados.
qs-sending-disconnected = Se cortó la conexión con { $label }. Se conservan los archivos ya comprobados. Intentándolo de nuevo solo…
qs-sending-needs-action =
    { NUMBER($count) ->
        [one] { $count } elemento necesita que hagas algo.
        [many] { $count } de elementos necesitan que hagas algo.
       *[other] { $count } elementos necesitan que hagas algo.
    }
qs-pause = Pausar el envío
qs-resume = Reanudar
qs-compact = Vista reducida
qs-stop = Parar este envío
qs-sending-complete-title = El envío ha terminado

## Q20 Stop this send? (and decline)

qs-stop-title = ¿Parar este envío?
qs-stop-body = Revisa lo que ya llegó antes de parar.
qs-stop-arrived-title = Archivos que llegaron completos
qs-stop-arrived-body = Se quedan en la carpeta que elegiste.
qs-stop-rest-title = Archivos aún sin enviar
qs-stop-rest-body = Después de parar no se envía nada más.
qs-stop-partial-title = Archivos que llegaron en parte
qs-stop-partial-body = Nunca se guardan con su nombre real, así que nunca se confunden con copias completas.
qs-stop-status-keep = Se conservan
qs-stop-status-stop = Se paran
qs-stop-status-review = Revisar
qs-stop-ack = Parar este envío
qs-stop-ack-hint = Los archivos que llegaron completos no se borran.
qs-stop-keep = Seguir enviando

## F08 Your moves, kept on this laptop (history)

history-title = Tus traslados, guardados en este portátil.
history-body = Abre de nuevo un informe o sigue con un traslado guardado.
history-row-move = { $from } → { $to }
history-status-finished = Completado
history-status-steps-left =
    { NUMBER($count) ->
        [one] Completado, te queda { $count } paso
        [many] Completado, te quedan { $count } de pasos
       *[other] Completado, te quedan { $count } pasos
    }
history-status-paused = En pausa
history-status-stopped = Parado sin riesgo
history-open-report = Informe
history-undo = Deshacer
history-continue = Continuar
history-export = Exportar historial
history-exported = Historial guardado como { $path }.
history-export-failed = No se pudo guardar el historial ahí. Elige otra carpeta e inténtalo de nuevo.
history-note = Este historial se queda en este portátil. No se envía nada a ningún sitio a menos que lo exportes y lo compartas tú.
history-empty = Tu primer traslado y su informe aparecerán aquí.
history-empty-title = Aún no hay traslados registrados
history-start = Empezar un traslado nuevo

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = Envío rápido todavía no está activado en esta versión. No se ha enviado ni recibido nada.
qs-tally-counting =
    { NUMBER($count) ->
        [one] { $count } elemento seleccionado · aún calculando el tamaño
        [many] { $count } de elementos seleccionados · aún calculando el tamaño
       *[other] { $count } elementos seleccionados · aún calculando el tamaño
    }
qs-gone = { $name } ya no está ahí. Puede que se haya movido o borrado. Desmárcalo, o vuelve a ponerlo en su sitio y añádelo de nuevo.
qs-choose-why-problems = Algunos elementos marcados aún no se pueden enviar. Desmárcalos o soluciónalos como se indica para continuar.
qs-sending-card = Enviando archivos
qs-progress-of = de { $total }
history-open-name = { $what }: { $title }

## R05 Deshacer: qué hará deshacer en cada disco, y cómo terminó cada copia

undo-here-removes = Deshacer elimina las copias que hizo PCTwin y que no has cambiado. Todo lo que cambiaste se conserva.
undo-here-to-trash = Deshacer mueve a la Papelera las copias que hizo PCTwin y que no has cambiado. Todo lo que cambiaste se conserva y no se elimina nada.
undo-here-keeps-fat = Deshacer conservará todas las copias en { $drive }. Este disco usa FAT o exFAT, que reutiliza los números que distinguen los archivos, así que PCTwin no puede estar seguro de cuáles son suyos. Puedes eliminarlos tú.
undo-here-keeps-other = Deshacer conservará todas las copias en { $drive }. En este tipo de disco, PCTwin no puede mover ni eliminar archivos de forma segura. Puedes eliminarlos tú.
undo-here-more = Más detalles
undo-here-more-trash = Un Mac no le permite a PCTwin saber que ninguna otra aplicación sigue escribiendo en un archivo, así que PCTwin nunca elimina ninguno. Mueve cada copia a la Papelera del mismo disco. Puedes recuperarla desde la Papelera.
undo-here-more-fat = En los discos FAT y exFAT, un archivo nuevo puede recibir el número que tenía un archivo eliminado. PCTwin solo actúa sobre archivos que puede demostrar que son suyos y que no han cambiado, así que conserva estos.
undo-done-trashed = Movido a la Papelera como «{ $name }». Puedes recuperarlo desde la Papelera.
undo-kept-fat = Conservado. Este disco (FAT o exFAT) reutiliza los números que distinguen los archivos, así que PCTwin no puede estar seguro de que sea su copia. Si no la necesitas, puedes eliminarla tú.
undo-kept-no-trash = Conservado. PCTwin no pudo mover esta copia a la Papelera de su disco. Si no la necesitas, puedes eliminarla tú.

## Quick Send (26 October 2026): pairing in Quick Send's words, where received files go, and the Done page

qs-pair-old-eyebrow = Enviando
qs-pair-old-body = En el portátil que recibe, elige Recibir archivos y luego escribe este código.
qs-pair-old-waiting = Esperando al portátil que recibe…
qs-pair-old-label-hint = El portátil que recibe encontrará el tuyo con este nombre.
qs-pair-old-steps-title = En el portátil que recibe
qs-pair-old-step-1 = Abre PCTwin, elige Envío rápido y luego Recibir archivos.
qs-pair-old-left = El portátil que recibe dejó de vincularse antes de elegir un número, así que no se compartió nada. Aquí tienes un código nuevo.
qs-pair-network-hint = Elige la red en la que está el portátil que recibe.
qs-pair-choose-body = Mira el portátil que recibe. Elige el número que muestra.
qs-pair-choose-renewed = Hay números nuevos. Elige el que aparece en el portátil que recibe.
qs-pair-choose-note = Si el portátil que recibe no muestra ningún número, o ninguno de estos coincide, elige Ninguno coincide.
qs-pair-heads-up-firewall = Windows puede preguntarte si PCTwin puede aceptar conexiones. Elige Permitir para que el portátil que recibe pueda llegar al tuyo.
qs-pair-new-eyebrow = Recibiendo
qs-pair-new-searching-title = Buscando el portátil que envía…
qs-pair-new-searching-body = Deja PCTwin abierto en el portátil que envía, en la misma Wi-Fi que el tuyo.
qs-pair-new-search-failed-title = No se pudo buscar el portátil que envía.
qs-pair-new-list-title = ¿Qué portátil está enviando?
qs-pair-new-list-body = Elige el nombre que muestra la pantalla del portátil que envía.
qs-pair-new-same-label = Otro portátil cercano tiene este nombre. Mira la pantalla del portátil que envía.
qs-pair-new-none-title = No aparece ningún portátil que envíe.
qs-pair-new-none-body = Comprueba que PCTwin esté abierto en el portátil que envía y muestre un código, y que los dos portátiles estén en la misma Wi-Fi. Luego busca de nuevo.
qs-pair-new-code-hint = Léelo en la pantalla de { $label }. Cambia cada minuto.
qs-pair-new-number-title = Elige este número en { $label }.
qs-pair-new-number-body = En { $label } aparecen tres números. Elige allí este.
qs-pair-new-number-waiting = Esperando a { $label }…
qs-pairing-code-invalid = El código debe tener seis cifras. Mira la pantalla de { $label } y escríbelo de nuevo.
qs-pairing-code-wrong = Ese código no coincide. Mira la pantalla de { $label } e inténtalo de nuevo.
qs-pairing-code-expired = Ese código ha cambiado. Escribe el nuevo que aparece en la pantalla de { $label }.
qs-pairing-busy = No se puede vincular con { $label } ahora: se está vinculando con otro portátil. Espera un momento e inténtalo de nuevo.
qs-pairing-wait = Se está preparando un código nuevo en { $label }. Espéralo y luego escribe el código nuevo.
qs-pairing-locked = La vinculación se detuvo tras demasiados intentos fallidos. En { $label }, elige Empezar de nuevo.
qs-pairing-unreachable = Tu portátil no pudo llegar a { $label }. Comprueba que los dos estén en la misma Wi-Fi e inténtalo de nuevo.
qs-pairing-wrong-number = Ese número no coincide con el del portátil que recibe. No se compartió nada. Escribe el código nuevo en el portátil que recibe para intentarlo de nuevo.
qs-pairing-no-match = Los números no coincidieron en { $label }. No se compartió nada. Escribe el código nuevo que aparece en su pantalla.
qs-pairing-timed-out = No se eligió ningún número a tiempo. Inténtalo de nuevo con el código nuevo de { $label }.
qs-pairing-connection-lost-new = Se cortó la conexión con { $label }. No se compartió nada.
qs-pairing-choose-network = Tu portátil está en más de una red. Elige la red del portátil que recibe:
qs-pairing-failed-new = El emparejamiento no funcionó esta vez. No se compartió nada. Escribe el código nuevo que aparece en la pantalla de { $label }.
qs-folder-line = Los archivos se guardarán en { $folder }.
qs-folder-change = Cambiar
qs-folder-title = ¿Dónde se guardan los archivos?
qs-folder-none = PCTwin no puede usar la carpeta de los archivos recibidos. Elige una carpeta para los archivos y luego conecta.
qs-folder-choose = Elegir una carpeta
qs-folder-failed = PCTwin no puede guardar archivos en esa carpeta. Elige otra.
qs-done-sent-title = Tus archivos se enviaron.
qs-done-here-title = Tus archivos ya están aquí.
qs-done-partial-title = Algunos archivos no llegaron.
qs-done-stopped-title = Este envío se detuvo.
qs-done-empty-title = No se envió nada.
qs-done-arrived =
    { NUMBER($count) ->
        [one] { $count } archivo · { $size } llegó e igual al original
        [many] { $count } de archivos · { $size } llegaron e iguales a los originales
       *[other] { $count } archivos · { $size } llegaron e iguales a los originales
    }
qs-done-arrived-count =
    { NUMBER($count) ->
        [one] { $count } archivo llegó e igual al original
        [many] { $count } de archivos llegaron e iguales a los originales
       *[other] { $count } archivos llegaron e iguales a los originales
    }
qs-done-already =
    { NUMBER($count) ->
        [one] { $count } archivo ya estaba ahí, idéntico, así que no se volvió a enviar.
        [many] { $count } de archivos ya estaban ahí, idénticos, así que no se volvieron a enviar.
       *[other] { $count } archivos ya estaban ahí, idénticos, así que no se volvieron a enviar.
    }
qs-done-stopped-body = Los archivos que llegaron antes de parar se comprobaron y siguen donde se guardaron. No se envió nada más.
qs-done-folder = Guardados en { $folder }
qs-done-open-folder = Abrir carpeta
qs-done-dates-new =
    { NUMBER($count) ->
        [one] Este archivo muestra la hora en que llegó, así que está arriba en la carpeta.
        [many] Estos { $count } de archivos muestran la hora en que llegaron, así que están arriba en la carpeta.
       *[other] Estos { $count } archivos muestran la hora en que llegaron, así que están arriba en la carpeta.
    }
qs-done-keep-dates = Conservar las fechas originales
qs-done-dates-kept =
    { NUMBER($count) ->
        [one] { $count } archivo tiene de nuevo su fecha original.
        [many] { $count } de archivos tienen de nuevo su fecha original.
       *[other] { $count } archivos tienen de nuevo su fecha original.
    }
qs-done-dates-failed = No se pudieron devolver las fechas originales. Los archivos están bien y siguen en su sitio.
qs-done-open-failed = No se pudo abrir la carpeta. Puede que se haya movido o borrado.
qs-done-not-opened = PCTwin no abrió ni puso en marcha ningún archivo recibido.
qs-done-problems-title = Archivos que necesitan atención
qs-done-not-arrived = No llegó.
qs-done-not-completed = No se envió antes de parar.
qs-done-needs-action = Está aquí, pero no terminado. Qué hacer está en Más detalles.
qs-done-mark-not-kept = Está aquí, pero esta unidad no pudo marcarlo como descargado de otro portátil. Ábrelo con cuidado.
qs-done-see-files = Ver los archivos
qs-done-renamed = Guardado como { $name }, porque ya había un archivo con ese nombre.
qs-done-identical = Ya había un archivo idéntico, así que no se volvió a enviar.
qs-done-send-missing = Enviar los archivos que faltan
qs-done-send-missing-body = Solo los archivos que no llegaron. Primero vuelves a vincular los portátiles.
qs-done-done = Listo
qs-stop-now = Parar el envío
qs-stop-finished = Este envío ya terminó.
qs-route-receiving = Recibiendo
qs-receiving-title = Los archivos están llegando.
qs-receiving-body = Las copias llegan directamente de { $label }.
qs-receiving-card = Recibiendo archivos
qs-receiving-progress =
    .aria-label = Recibiendo archivos
qs-receiving-current = Recibiendo: { $name }
qs-step-receiving-body =
    { NUMBER($done) ->
        [one] { $done } archivo llegó.
        [many] { $done } de archivos llegaron.
       *[other] { $done } archivos llegaron.
    }
qs-tally-files =
    { NUMBER($count) ->
        [one] { $count } archivo · { $size }
        [many] { $count } de archivos · { $size }
       *[other] { $count } archivos · { $size }
    }
qs-receive-pause = Pausar
qs-label-yours = Tu portátil
qs-label-receiving = Portátil que recibe
qs-label-sending = Portátil que envía
qs-peer-receiving = el portátil que recibe
qs-peer-sending = el portátil que envía
qs-pair-old-label-title = Nombre de tu portátil
qs-pair-old-rename-label = Nombre para tu portátil
qs-pair-old-step-2 = Si aparece una lista, elige el nombre de tu portátil.
qs-pair-old-locked-body = Tras cinco códigos erróneos, la vinculación se detuvo para proteger tu portátil. No se compartió nada. Si fuiste tú, elige Empezar de nuevo para obtener un código nuevo.
qs-pair-new-needs-update = Tiene una versión antigua de PCTwin. Actualiza PCTwin en él y busca de nuevo.
qs-pairing-no-network = Tu portátil no está en ninguna red Wi-Fi ni por cable. Conéctalo e inténtalo de nuevo.
qs-problem-room-title = No hay espacio suficiente en { $folder }.
qs-problem-room-body = Estos archivos necesitan { $needed }, y allí se pueden usar { $room }. No se guardó nada. Libera espacio o elige otra carpeta e inténtalo de nuevo.
qs-done-refused-title = El portátil que recibe no aceptó estos archivos.
qs-done-refused-body = Puede que no tenga espacio suficiente. No se envió nada.

## Pantallas de la fase 2 (lista de tareas 2.8): partes comunes

guard-state-not-started = Aún no ha empezado
guard-state-preparing = Preparando
guard-state-awaiting-person = Esperando tu elección
guard-state-awaiting-system = Esperando la aprobación de tu sistema
guard-state-awaiting-maker = Esperando al creador de la aplicación o del servicio en línea
guard-state-working = Trabajando
guard-state-checking = Comprobando lo que se hizo
guard-state-applying = Poniendo las cosas en su sitio
guard-state-pause-requested = Se pausará en el próximo punto seguro. Todavía no está en pausa.
guard-state-paused = En pausa en un punto seguro
guard-state-source-unavailable = El portátil antiguo o el disco no está conectado
guard-state-blocked = Primero hay que arreglar algo
guard-state-partial = Hecho en parte
guard-state-retrying = Intentando de nuevo
guard-state-reconciling = Comprobando hasta dónde se llegó
guard-state-failed = Esto no funcionó
guard-state-complete = Listo
guard-state-cancelled = Detenido. Lo que ya se hizo se conserva.
guard-refused-unavailable = PCTwin no pudo comprobar esto ahora mismo. No se hizo nada. Inténtalo de nuevo en un momento.
guard-refused-wrong-job = Esto era de otro traslado. No se hizo nada.
guard-refused-old-session = Esta pantalla estaba desactualizada, así que no se hizo nada. Esto es lo que es cierto ahora.
guard-refused-plan-changed = El plan cambió mientras lo mirabas, así que no se hizo nada. Míralo de nuevo.
guard-refused-not-now = Eso no se puede hacer en este paso. No se hizo nada.
guard-refused-not-shown = Esa opción ya no está en esta pantalla. No se hizo nada.
guard-progress-label = Progreso
guard-progress-mark = { $percent } % completado
guard-progress-of = { $done } de { $total }
guard-progress-unknown-total = { $done } hasta ahora. Aún no se sabe el total.
guard-progress-none = Todavía no se ha medido nada.
guard-and-more =
    { $count ->
        [one] Y { $count } más, en el informe.
        [many] Y { $count } más, en el informe.
       *[other] Y { $count } más, en el informe.
    }
guard-not-known-yet = Aún no se sabe
guard-free = { $free } libres
guard-free-unknown = Espacio libre desconocido
guard-items-size = { $items } elementos, { $size }
guard-try-again = Intentar de nuevo
guard-pause = Pausar
guard-resume = Seguir
guard-check-again = Comprobar de nuevo
guard-strip-plan = Tu plan
guard-strip-report = Tu informe
guard-strip-permission = Aprobación
guard-strip-rescue = Rescate
guard-strip-undo = Deshacer
guard-strip-checking = Comprobando

## X09 comprobación tras un reinicio, y estado no disponible

guard-checking-title = Comprobando hasta dónde se llegó
guard-checking-body = PCTwin está mirando lo que estaba terminado antes de cerrarse. Tus opciones vuelven en cuanto lo sepa.
guard-unavailable-title = Estado no disponible
guard-unavailable-body = PCTwin no puede decir ahora mismo qué está haciendo este traslado. No se muestra nada como hecho hasta que pueda. Vuelve a comprobar en un momento.

## 2.9 Antes de cambiar nada

guard-changes-title = Antes de cambiar nada
guard-changes-body = Esto es todo lo que el traslado cambiará en este portátil, y lo que deshacer puede hacer con cada cosa. Todavía no ha cambiado nada.
guard-changes-download = Descargar de internet
guard-changes-download-none = Nada
guard-changes-permissions = Aprobación del sistema
guard-changes-one-prompt = Tu sistema pregunta una sola vez
guard-changes-no-prompt = No hace falta
guard-changes-restore-point = Punto de restauración de Windows
guard-changes-restore-point-first = Se pide primero
guard-changes-restore-point-none = No se usa
guard-changes-install =
    { $count ->
        [one] Por instalar: { $count } aplicación
        [many] Por instalar: { $count } de aplicaciones
       *[other] Por instalar: { $count } aplicaciones
    }
guard-changes-merge =
    { $count ->
        [one] Se añade junto a lo que hay aquí: { $count } elemento
        [many] Se añaden junto a lo que hay aquí: { $count } de elementos
       *[other] Se añaden junto a lo que hay aquí: { $count } elementos
    }
guard-changes-replace =
    { $count ->
        [one] Por reemplazar, conservando la versión anterior: { $count } elemento
        [many] Por reemplazar, conservando las versiones anteriores: { $count } de elementos
       *[other] Por reemplazar, conservando las versiones anteriores: { $count } elementos
    }
guard-changes-leave-alone =
    { $count ->
        [one] Se deja como está: { $count } elemento
        [many] Se dejan como están: { $count } de elementos
       *[other] Se dejan como están: { $count } elementos
    }
guard-changes-space = Espacio en cada disco en el momento de más uso
guard-changes-why-prompt = Por qué preguntará tu sistema
guard-changes-approve = Aprobar estos cambios
guard-undo-legend-can = PCTwin puede deshacerlo.
guard-undo-legend-keeps = PCTwin lo conserva. Deshacer no lo toca.
guard-undo-legend-you = Te toca a ti. Solo su servicio o su creador puede deshacerlo.
guard-undo-can-reverse = PCTwin puede deshacerlo
guard-undo-can-remove-app = PCTwin puede quitar la aplicación. No toca los archivos que creaste.
guard-undo-must-keep = Se conserva: ya estaba aquí
guard-undo-needs-you-service = Te toca a ti: solo el servicio en línea puede deshacerlo
guard-undo-needs-you-maker = Te toca a ti: los inicios de sesión y las licencias son del creador
guard-drive-needs = Necesita { $needed }. { $free } libres.
guard-drive-needs-free-unknown = Necesita { $needed }. El disco no dice cuánto hay libre.
guard-drive-fits = Cabe
guard-drive-short = Faltan { $short }
guard-drive-unknown = No se sabe
guard-permission-read-other-people = Para leer los archivos de otras personas en el portátil antiguo
guard-permission-make-accounts = Para crear sus cuentas en este portátil
guard-permission-shared-folders = Para poner cosas en carpetas compartidas que tu cuenta no puede cambiar
guard-permission-install-for-everyone = Para instalar aplicaciones para todos en este portátil
guard-permission-restore-point = Para crear primero un punto de restauración de Windows

## X06 y H10 algo cambió después de la aprobación

guard-changed-title =
    { $count ->
        [one] { $count } cosa cambió después de que aprobaras
        [many] { $count } de cosas cambiaron después de que aprobaras
       *[other] { $count } cosas cambiaron después de que aprobaras
    }
guard-changed-body = Solo estas necesitan que las mires de nuevo. Todo lo demás sigue aprobado y continúa.
guard-changed-changed-on-old-laptop = Cambió en el portátil antiguo.
guard-changed-app-updated = La aplicación se actualizó sola.
guard-changed-destination-changed = Cambió el lugar al que va.
guard-changed-new-scope = Es nuevo: otra persona u otro portátil se unió al traslado.
guard-changed-permission-ended = La aprobación para esto terminó.
guard-changed-leave-out = Dejar estas fuera
guard-changed-approve = Aprobar estas de nuevo

## M31, M05 espacio en el portátil nuevo

guard-space-title = Espacio en el portátil nuevo
guard-space-title-short = Al portátil nuevo le falta espacio
guard-space-body = PCTwin contó todo lo que llega a cada disco, incluido el espacio para deshacer. No se descarta nada de lo que elegiste para que quepa.
guard-space-proposal = Lo que PCTwin sugiere para lo que no cabe
guard-keep-off-leave-in-cloud = Se dejan en su nube, donde están seguros
guard-keep-off-offload-drive = Se ponen en tu otro disco
guard-keep-off-leave-on-old-laptop = Se dejan en el portátil antiguo. Consérvalo hasta que se trasladen.
guard-offload-title = Tu otro disco
guard-offload-connected = Conectado
guard-offload-not-connected = No conectado
guard-offload-approved = Elegiste este disco. Los archivos que hay en él no están en el portátil nuevo: conéctalo para abrirlos.
guard-offload-ask = PCTwin puede poner algunas cosas en este disco. Necesita tu sí para este disco en concreto.
guard-offload-use = Usar este disco
guard-space-approve = Aprobar este plan
guard-safe-in = A salvo en { $provider }
guard-safe-in-cloud = A salvo en tu nube
guard-safe-in-detail =
    { $count ->
        [one] { $count } archivo está guardado solo en línea. Vuelve cuando inicies sesión en el portátil nuevo.
        [many] { $count } de archivos están guardados solo en línea. Vuelven cuando inicies sesión en el portátil nuevo.
       *[other] { $count } archivos están guardados solo en línea. Vuelven cuando inicies sesión en el portátil nuevo.
    }
guard-your-cloud = tu nube

## M09 conservar una copia de los archivos que solo están en línea

guard-local-title = ¿Conservar una copia de tus archivos que solo están en línea?
guard-local-body =
    { $count ->
        [one] { $count } archivo de { $provider } está guardado solo en línea. También puedes conservar una copia en el portátil nuevo.
        [many] { $count } de archivos de { $provider } están guardados solo en línea. También puedes conservar una copia en el portátil nuevo.
       *[other] { $count } archivos de { $provider } están guardados solo en línea. También puedes conservar una copia en el portátil nuevo.
    }
guard-local-download = Por descargar en el portátil antiguo
guard-local-old-space = Espacio en el portátil antiguo
guard-local-how = PCTwin los baja unos pocos cada vez, los envía y luego vuelve a dejarlos solo en línea en el portátil antiguo.
guard-local-paused-metered = En pausa: esta conexión puede cobrar por los datos. Sigue sola con una conexión gratuita, o ahora mismo si lo permites.
guard-local-paused-no-space = En pausa: el portátil antiguo se ha quedado sin espacio para los próximos archivos. El resto sigue a salvo en { $provider }.
guard-local-done = Listo. Las copias están en el portátil nuevo.
guard-local-keep = Conservar una copia
guard-local-leave = Dejarlos en { $provider }
guard-local-allow-metered = Permitir en esta conexión

## M07 quién va adónde

guard-owners-title = Quién va adónde
guard-owners-body = Las cosas de cada persona van a su propia cuenta en este portátil. Que los nombres coincidan nunca decide nada por sí solo.
guard-owners-from = De { $laptop }
guard-owners-to-account = A la cuenta { $account } de este portátil
guard-owners-new-account = A una cuenta nueva de este portátil
guard-owners-own-folder = Sus propias carpetas «De { $laptop }», porque los nombres coinciden
guard-owners-usual-folders = Sus carpetas de siempre
guard-owners-approve = Aprobar

## X12 lo que no se terminó

guard-report-title-partial = Casi todo se trasladó. Unas pocas cosas no.
guard-report-title-done = Todo lo de esta parte se trasladó.
guard-report-body-partial = Todo lo de abajo sigue en el portátil antiguo. Inténtalo de nuevo con cada cosa cuando quieras. Lo que ya se trasladó se queda.
guard-report-body-done = No queda nada por intentar de nuevo.
guard-line-app = La aplicación { $name }
guard-line-items =
    { $count ->
        [one] { $count } elemento
        [many] { $count } de elementos
       *[other] { $count } elementos
    }
guard-line-settings =
    { $count ->
        [one] { $count } ajuste
        [many] { $count } de ajustes
       *[other] { $count } ajustes
    }
guard-line-why-failed = De { $laptop }: no se terminó.
guard-line-why-laptop-not-connected = De { $laptop }: ese portátil no está conectado. Conéctalo a la misma red Wi-Fi y mantenlo encendido.
guard-line-why-waits-for-another = De { $laptop }: espera a otro paso que no se terminó. Arregla primero ese.
guard-line-why-not-started = De { $laptop }: el traslado se detuvo antes de empezar.

## M25, H10 la aprobación del sistema, y el punto de restauración

guard-permission-title-explain = PCTwin necesita una aprobación de tu sistema
guard-permission-title-waiting-for-system = Responde a la pregunta de tu sistema
guard-permission-title-denied = Dijiste que no
guard-permission-title-closed-without-answer = La pregunta se cerró sin respuesta
guard-permission-title-given = Aprobado para este traslado
guard-permission-title-ended-by-sleep = La aprobación terminó cuando el portátil se suspendió
guard-permission-title-ended-other-session = La aprobación terminó cuando otra persona inició sesión
guard-permission-body-explain = Tu sistema pregunta una sola vez para todo el traslado. Solo la usan los pasos que se indican abajo, y termina con el traslado.
guard-permission-body-waiting-for-system = La ventana de tu propio sistema está abierta. Escribe allí tu contraseña si la pide.
guard-permission-body-denied = Solo se dejan fuera los pasos que la necesitaban. Todo lo demás continúa.
guard-permission-body-closed-without-answer = Cerrarla no es un sí, así que no se hizo nada de lo que la necesita. Pregunta de nuevo, o sigue sin ella.
guard-permission-body-given = PCTwin la usa solo para los pasos que se indican abajo, y termina con este traslado.
guard-permission-body-ended-by-sleep = Después de eso no se hizo nada de lo que la necesita. Pregunta de nuevo para seguir.
guard-permission-body-ended-other-session = Después de eso no se hizo nada de lo que la necesita. Pregunta de nuevo cuando vuelvas a tu cuenta.
guard-permission-never-password = PCTwin nunca te pide la contraseña y nunca la ve. Solo la ve tu sistema.
guard-permission-not-now = Seguir sin ella
guard-permission-ask = Continuar con tu sistema
guard-permission-ask-again = Preguntar de nuevo
guard-restore-title = Punto de restauración de Windows
guard-restore-made = Creado a las { $time }.
guard-restore-earlier-exists = Windows ya tiene uno de { $time }, así que no creó otro.
guard-restore-not-chosen = No se creó: elegiste no hacerlo.
guard-restore-not-made-not-windows = No se creó: los puntos de restauración solo existen en Windows.
guard-restore-not-made-protection-off = No se creó: System Protection está desactivada para este disco. Puedes activarla en System Properties.
guard-restore-not-made-needs-admin = No se creó: necesita la aprobación de tu sistema.
guard-restore-not-made-unconfirmed = No se creó: Windows dijo que sí, pero no se encuentra ningún punto de restauración.
guard-restore-not-made-failed = No se creó: Windows no pudo crear uno.

## T01 a T04 rescate de un portátil que no arranca

guard-rescue-title-intro = Salva lo importante de un portátil que no arranca
guard-rescue-title-pick-drive = Elige el disco que quieres rescatar
guard-rescue-title-locked = Este disco está bloqueado
guard-rescue-title-no-system = Este no es el disco de un portátil
guard-rescue-title-salvage = Esto es lo que se pudo leer
guard-rescue-body-intro = Saca el disco del portátil antiguo y conéctalo a este con un adaptador o una carcasa para discos.
guard-rescue-body-pick-drive = Elige el disco del portátil antiguo. El disco al que van las cosas no puede ser el que rescatas.
guard-rescue-body-locked = Desbloquéalo primero en tu sistema. Después, comprueba de nuevo.
guard-rescue-body-no-system = Este disco no tiene el sistema de un portátil ni carpetas personales. Si es un disco USB normal, úsalo mejor en el portátil nuevo.
guard-rescue-body-salvage = Revisa la lista y usa lo que se pudo leer.
guard-rescue-can = Lo que puede hacer un rescate
guard-rescue-can-body = Leer tus archivos, y las personas y los ajustes que haya en el disco, de un disco que todavía funciona.
guard-rescue-cannot = Lo que no puede hacer
guard-rescue-cannot-body = Reparar el disco, leer las partes dañadas o abrir un disco cuya clave no tienes.
guard-rescue-read-only-title = El disco solo se lee
guard-rescue-read-only-promise = PCTwin solo lo lee. No se cambia, repara ni formatea nada en él.
guard-rescue-system-read-only = Tu sistema lo abrió en solo lectura, así que nada en él puede cambiar.
guard-rescue-pctwin-reads-only = PCTwin solo lo leyó. Tu sistema lo tenía abierto para hacer cambios, así que otras aplicaciones aún podían cambiarlo.
guard-rescue-refused-target = Ese disco es donde se ponen las cosas, así que no se puede rescatar también. Elige otro disco.
guard-rescue-drive-detail = { $size }, { $fs }
guard-rescue-drive-is-target = Las cosas del traslado van a este disco.
guard-rescue-drive-cannot = No se puede rescatar
guard-rescue-choose = Rescatar este disco
guard-rescue-bit-locker-step-1 = Abre el disco en File Explorer.
guard-rescue-bit-locker-step-2 = Escribe su clave de recuperación cuando Windows la pida. Suele estar guardada en la cuenta de Microsoft del propietario, en aka.ms/myrecoverykey.
guard-rescue-bit-locker-step-3 = Vuelve aquí y elige Comprobar de nuevo.
guard-rescue-file-vault-step-1 = Desbloquéalo en un Mac con la contraseña de inicio de sesión del propietario o su clave de recuperación.
guard-rescue-file-vault-step-2 = Vuelve aquí y elige Comprobar de nuevo.
guard-rescue-apple-chip-step-1 = Este Mac mantiene su disco bloqueado solo para él, así que sacar el disco no sirve.
guard-rescue-apple-chip-step-2 = Si el Mac todavía arranca en Recovery, usa Share Disk con un cable. Si no arranca en absoluto, solo un centro de reparación de Apple puede ayudar.
guard-rescue-never-key = PCTwin nunca pide la clave y nunca la guarda.
guard-rescue-readable =
    { $count ->
        [one] Se puede leer { $n } elemento.
        [many] Se pueden leer { $n } de elementos.
       *[other] Se pueden leer { $n } elementos.
    }
guard-rescue-unreadable =
    { $count ->
        [one] No se pudo leer { $count } elemento
        [many] No se pudieron leer { $count } de elementos
       *[other] No se pudieron leer { $count } elementos
    }
guard-rescue-look = Buscar el disco
guard-rescue-check-again = Comprobar de nuevo
guard-rescue-use = Usar lo que se pudo leer

## R05, R06 deshacer

guard-undo-preview-title = Lo que hará deshacer
guard-undo-preview-body = PCTwin acaba de mirar cada cambio. Todo lo que cambiaste después del traslado se conserva. No pasa nada hasta que elijas Deshacer.
guard-undo-preview-which = Esto deshace el traslado que terminó el { DATETIME($when, year: "numeric", month: "long", day: "numeric", hour: "numeric", minute: "2-digit") }. Tus otros traslados no cambian.
guard-undo-preview-which-earlier = Esto deshace el traslado hecho con una versión anterior de PCTwin. Tus otros traslados no cambian.
guard-undo-preview-admin = Para quitar aplicaciones, esta laptop pedirá la aprobación de un administrador.
guard-undo-plan-put-back = Se deja como estaba
guard-undo-plan-remove-app = PCTwin quita la aplicación. PCTwin no toca los archivos que creaste.
guard-undo-plan-keep-changed = Se conserva: lo cambiaste
guard-undo-plan-elsewhere = Te toca a ti: se hace en otro sitio
guard-undo-plan-leave-alone = Se deja como está: ya estaba aquí
guard-undo-keep = Dejarlo todo como está
guard-undo-run = Deshacer
guard-undo-running-title = Deshaciendo los cambios
guard-undo-done-title = Lo que hizo deshacer
guard-undo-result-restored = Restaurado
guard-undo-result-removed = Quitado
guard-undo-result-as-before = Ya estaba como antes
guard-undo-result-kept-changed = Se conserva porque lo cambiaste
guard-undo-result-kept-other = Se conserva, mira Más detalles
guard-undo-result-kept-at = Se conserva como { $name }
guard-undo-result-done-elsewhere = Hecho en otro sitio: solo su servicio o su creador puede deshacerlo
guard-undo-result-not-done-yet = Aún no se hizo. Deshacer lo intentará la próxima vez.

## The full move: apps, settings and app data (28 October 2026)

fullmove-plan-title = Apps, ajustes y datos de apps
fullmove-plan-body = PCTwin revisó las apps, los ajustes y los datos de apps de tu laptop anterior. Esto es lo que hará en esta laptop después de tus archivos. Nada cambia hasta que empieces la mudanza.
fullmove-group-install = Apps que PCTwin instalará: { $count }
fullmove-group-settings = Ajustes de apps que PCTwin copiará: { $count }
fullmove-group-needs = Cosas que harás tú: { $count }
fullmove-group-here = Apps que ya están en esta laptop: { $count }
fullmove-settings-count = Ajustes: { $count }.
fullmove-settings-merge = Se añaden. No se reemplaza nada en esta laptop.
fullmove-settings-replace = Se reemplazan algunos ajustes de esta laptop. Deshacer puede devolverlos.
fullmove-app-install = PCTwin la instala desde su creador y la comprueba.
fullmove-app-here = Ya está en esta laptop, así que PCTwin la deja como está.
fullmove-step-copy = Copiado, con opción de deshacer.
fullmove-step-export-first = En tu laptop anterior, usa la exportación propia de la app. Después impórtala en la app de esta laptop.
fullmove-step-sign-in = Vuelve a iniciar sesión en esta laptop.
fullmove-step-relink = Configúrala de nuevo en esta laptop o vuelve a vincularla desde la app.
fullmove-step-not-yet = PCTwin todavía no mueve los datos de esta app. Siguen a salvo en tu laptop anterior.
fullmove-step-look-at-it = PCTwin no pudo saber qué es esto. Revísalo en tu laptop anterior.
fullmove-group-update =
    { $count ->
        [one] Hay una versión anterior en este portátil
        [many] Hay versiones anteriores en este portátil
       *[other] Hay versiones anteriores en este portátil
    }
fullmove-update-it = Actualizar { $name }
fullmove-update-hint = PCTwin instala la versión más nueva. Tus ajustes se conservan.
fullmove-special-not-yet = Datos de apps como correo, navegadores y juegos
fullmove-left-by-person = Aplicaciones, ajustes y datos de aplicaciones: se quedan en tu portátil antiguo, como elegiste
fullmove-special-finish = PCTwin no movió esto. Sigue a salvo en tu laptop anterior.
fullmove-more = Y { $count } más.
fullmove-not-moved-title = Ajustes que PCTwin deja en la laptop anterior
fullmove-not-moved-private = Ajustes privados y de seguridad, e inicios de sesión, que nunca se copian: { $count }
fullmove-not-moved-could-run = Ajustes que iniciarían o abrirían algo, retenidos por seguridad: { $count }
fullmove-not-moved-old-laptop-only = Ajustes que solo sirven en la laptop anterior: { $count }
fullmove-not-moved-system-own = Ajustes propios del sistema que PCTwin todavía no mueve: { $count }
fullmove-not-moved-not-read = Ajustes encontrados pero no leídos: { $count }
fullmove-special-not-moved = Archivos de apps que se quedan, como cachés: { $count }
fullmove-prompt = Esta laptop pedirá una vez el permiso de un administrador.
fullmove-prompt-restore = Esta laptop pedirá una vez el permiso de un administrador y creará un punto de restauración antes del primer cambio.
fullmove-rebuilding-title = Preparando tus apps y ajustes
fullmove-rebuilding-body = Tus archivos ya están en su sitio. PCTwin está preparando el resto. Mantén esta laptop encendida.
fullmove-rebuilding-permission = Pidiendo permiso a esta laptop
fullmove-rebuilding-apps = Apps: { $done } de { $total }
fullmove-rebuilding-settings = Ajustes: { $done } de { $total }
fullmove-outcome-title = Apps, ajustes y datos de apps
fullmove-outcome-undo = Deshacer ajustes de apps
fullmove-outcome-stopped = PCTwin dejó de cambiar ajustes a medias porque no pudo anotar su siguiente cambio. No se cambió nada después de ese punto.
fullmove-restore-made = Se creó un punto de restauración antes de los cambios.
fullmove-restore-earlier-exists = No se creó un punto de restauración nuevo porque esta laptop ya tenía uno de hoy.
fullmove-restore-not-made = No se creó un punto de restauración. Deshacer de PCTwin sigue funcionando.
fullmove-restore-not-windows = Esta laptop no tiene puntos de restauración. Deshacer de PCTwin sigue funcionando.
guard-undo-remove-yourself = Quítala tú si quieres
guard-undo-plan-remove-yourself = Puedes quitarla en la configuración de tu laptop
guard-undo-result-remove-yourself = Queda para que la quites en la configuración de tu laptop
guard-undo-plan-keep-other-version = Se conserva: cambió desde el traslado
guard-undo-plan-already-gone = Ya se quitó
guard-undo-result-kept-other-version = Se conservó porque cambió desde el traslado
guard-undo-result-removed-restart = Quitada. Reinicia tu laptop para terminar.
guard-undo-result-not-removed = No se quitó. Puedes quitarla en la configuración de tu laptop.
guard-undo-close = Listo
move-wait-appdata-title = Revisando los datos de tus apps.
move-wait-appdata-body = Tus archivos ya están encontrados. PCTwin sigue revisando los datos que guardan tus apps, como el correo y los navegadores. Tarda como mucho diez minutos. No se cambia nada en esta laptop.
fullmove-special-not-checked = Datos de apps que PCTwin no pudo revisar del todo
fullmove-group-appdata = Datos de apps que PCTwin copiará: { $count }
fullmove-group-secrets = Archivos secretos que puedes elegir traer: { $count }
fullmove-secrets-body = Contienen contraseñas o claves. PCTwin solo trae uno si lo marcas tú. Sin marcar, se queda en tu laptop anterior.
fullmove-secret-bring = Traer { $name }
fullmove-secret-hint = Solo tú podrás abrirlo en esta laptop. Deshacer puede quitarlo de nuevo.
fullmove-appdata-title = Copiando los datos de tus apps
fullmove-appdata-progress = Copiando datos de apps: { $done } de { $total } apps
fullmove-appdata-waiting = Esperando a la laptop anterior: cierra { $apps } allí.
fullmove-close-title = Cierra estas apps para traer sus datos
fullmove-close-body = Estas apps siguen abiertas en esta laptop, así que PCTwin no puede copiar sus datos con seguridad. Ciérralas y elige Intentar de nuevo. Si sigues sin ellas, sus datos se quedan aquí.
fullmove-close-again = Intentar de nuevo
fullmove-close-without = Seguir sin ellas
fullmove-extras-refused = No se pudieron leer aquí los ajustes y datos de apps de la laptop anterior, así que no se movió ninguno. Siguen en tu laptop anterior.
fullmove-app-from = PCTwin instala el paquete de winget { $id } de su creador, tras comprobar que es esta app y el mismo creador.
fullmove-here-older = En esta laptop hay una versión anterior ({ $version }). PCTwin la deja como está.
fullmove-step-sites = Los inicios de sesión guardados en sitios web siguen ligados a la laptop anterior. Vuelve a iniciar sesión en esos sitios en esta laptop.
fullmove-links = Accesos directos a otros lugares entre tus archivos, que PCTwin no sigue ni traslada: { $count }
fullmove-note-sign-in = Ábrela e inicia sesión en tu cuenta.
fullmove-note-licence-key = Ábrela e introduce tu clave de licencia, o inicia sesión en la cuenta que tiene tu licencia.
fullmove-note-registration = Tu laptop anterior la tenía registrada a nombre de { $who }.
fullmove-note-product-id = Su ID de producto en tu laptop anterior era { $id }.
fullmove-note-extras = Llegaron otros programas con ella: { $apps }. Quítalos en la configuración de tu laptop si no los quieres.
fullmove-data-lost = Se perdió la conexión antes de que llegaran estos datos. Siguen a salvo en tu laptop anterior. Inicia un nuevo traslado para traerlos.
fullmove-data-did-not-arrive = Estos datos no llegaron. Siguen a salvo en tu laptop anterior. Inicia un nuevo traslado para traerlos.
fullmove-data-app-open = La app estaba abierta en tu laptop anterior, así que sus datos no se copiaron. Cierra la app allí y luego inicia un nuevo traslado para traerlos.
fullmove-data-changing = Estos datos cambiaban en tu laptop anterior mientras PCTwin los leía. Cierra la app allí y luego inicia un nuevo traslado para traerlos.
fullmove-data-unreadable = PCTwin no pudo leer estos datos en tu laptop anterior. Siguen allí.
fullmove-data-not-read = El traslado se detuvo antes de leer estos datos. Siguen a salvo en tu laptop anterior.
fullmove-data-not-whole = No llegaron todos estos datos, así que PCTwin no colocó ninguno. Siguen a salvo en tu laptop anterior.
fullmove-data-not-landed = Estos datos llegaron, pero PCTwin no pudo colocarlos en esta laptop. Siguen a salvo en tu laptop anterior.
fullmove-data-beside = La app ya tenía su propia carpeta en esta laptop, así que PCTwin puso la copia de la laptop anterior al lado, en { $folder }. La app sigue usando su propia carpeta. Para usar la copia anterior, cierra la app e intercambia las dos carpetas.
fullmove-data-placed = Los datos están en { $folder }, pero la app necesita un paso antes de poder usarlos. Abre la app y revísalos.
fullmove-outcome-here-title = Apps que ya están en esta laptop: { $count }
fullmove-outcome-here = Ya está en esta laptop, se deja como está.
fullmove-outcome-here-older = En esta laptop hay una versión anterior ({ $version }). PCTwin la dejó como está. Actualízala desde su creador si quieres la más nueva.
fullmove-outcome-here-badge = Se deja como está
fullmove-outcome-fresh = Las apps que instaló PCTwin empiezan desde cero. Abre cada una e inicia sesión si te lo pide.
progress-may-sleep = Esta laptop podría suspenderse durante el traslado. Mantenla conectada y abierta.

## The old laptop chooses what moves (decided 28 October 2026)
choose-title = ¿Qué quieres mover?
choose-body = Elige aquí, en el portátil donde están tus cosas. Tu nuevo portátil te mostrará lo que llega y te pedirá que lo aceptes.
choose-mode = Qué mover
choose-everything = Todo
choose-except = Todo excepto
choose-only = Solo lo que elija
choose-except-hint = Desmarca lo que quieras dejar.
choose-open = Abrir { $name }
choose-close = Cerrar { $name }
choose-total =
    { $files ->
        [one] 1 archivo
        [many] { $files } archivos
       *[other] { $files } archivos
    } · Elegido: { $size }
choose-only-hint = Marca lo que quieras llevar.
choose-apps = Aplicaciones, ajustes y datos de aplicaciones
choose-apps-count =
    { $count ->
        [one] 1 aplicación, con sus ajustes
        [many] { $count } aplicaciones, con sus ajustes
       *[other] { $count } aplicaciones, con sus ajustes
    }
choose-group-size =
    { $files ->
        [one] 1 archivo
        [many] { $files } archivos
       *[other] { $files } archivos
    } · { $size }
choose-left-title = Se quedan de todos modos
choose-left-badge = Se queda
choose-left-caches = Cachés y archivos temporales
choose-left-chosen-by-user = Lo que elegiste dejar
choose-left-work-managed = Archivos de tu trabajo o escuela
choose-left-device-bound = Cosas ligadas a este portátil, como las llaves de acceso
choose-left-unreadable = Archivos que PCTwin no pudo leer
choose-left-cloud-only = Archivos guardados solo en línea
choose-left-system = Partes del propio Windows
choose-left-too-many-files = Carpetas demasiado grandes para un solo traslado
choose-left-rebuildable = Cosas que tu nuevo portátil puede volver a crear, como paquetes descargados
choose-nothing = Elige al menos una cosa para mover.
choose-continue = Continuar

## The new laptop accepts what the old laptop chose (decided 28 October 2026)
accept-title = { $from } quiere enviarte esto.
accept-body = Se eligió en tu portátil antiguo. Acéptalo y PCTwin lo trae aquí.
accept-files =
    { $count ->
        [one] 1 archivo
        [many] { $count } archivos
       *[other] { $count } archivos
    }
accept-apps =
    { $count ->
        [one] 1 aplicación, instalada para ti
        [many] { $count } aplicaciones, instaladas para ti
       *[other] { $count } aplicaciones, instaladas para ti
    }
accept-settings = Ajustes y datos de aplicaciones
accept-fits = Cabe en este portátil.
accept-needs = Algo necesita tu atención primero. Mira los detalles.
accept-not-now = Ahora no
accept-details = Ver los detalles
accept-accept = Aceptar

# 2.6 wiring
reason-open-in-app = Una aplicación tenía este archivo abierto en el portátil antiguo, así que PCTwin no lo copió (una copia hecha mientras la aplicación escribe podría quedar dañada). Cierra esa aplicación en el portátil antiguo y luego inicia un traslado nuevo que incluya este archivo.
report-open-in = Abierto en { $app }
fullmove-close-files = Estos archivos están abiertos en una aplicación de este portátil, así que PCTwin todavía no los copiará: { $files }. Cierra la aplicación que los usa y luego elige Intentar de nuevo.
choose-online-only =
    { NUMBER($count) ->
        [one] Traer también el archivo guardado solo en línea en { $provider } (1 archivo, { $size }). Primero se descarga en este portátil y después vuelve a quedar solo en línea.
        [many] Traer también los archivos guardados solo en línea en { $provider } ({ $count } de archivos, { $size }). Primero se descargan en este portátil y después vuelven a quedar solo en línea.
       *[other] Traer también los archivos guardados solo en línea en { $provider } ({ $count } archivos, { $size }). Primero se descargan en este portátil y después vuelven a quedar solo en línea.
    }
choose-online-only-unnamed = una carpeta en la nube de este portátil
choose-online-only-metered = También con una conexión que puede cobrar por los datos (la zona Wi-Fi de un teléfono)
choose-online-only-not-here = Los archivos guardados solo en línea siguen a salvo en línea. Traer una copia de ellos todavía no está disponible en un Mac.
kept-online-copied =
    { NUMBER($count) ->
        [one] Se descargó y se copió 1 archivo guardado solo en línea.
        [many] Se descargaron y se copiaron { $count } de archivos guardados solo en línea.
       *[other] Se descargaron y se copiaron { $count } archivos guardados solo en línea.
    }
kept-online-safe =
    { NUMBER($count) ->
        [one] 1 archivo guardado solo en línea no se copió y sigue a salvo en línea. Ábrelo en el portátil nuevo desde la aplicación que lo guarda.
        [many] { $count } de archivos guardados solo en línea no se copiaron y siguen a salvo en línea. Ábrelos en el portátil nuevo desde la aplicación que los guarda.
       *[other] { $count } archivos guardados solo en línea no se copiaron y siguen a salvo en línea. Ábrelos en el portátil nuevo desde la aplicación que los guarda.
    }
kept-online-still-here =
    { NUMBER($count) ->
        [one] 1 archivo descargado todavía ocupa { $size } en este portátil, porque su aplicación de la nube no lo ha vuelto a dejar solo en línea. Para liberar el espacio, haz clic derecho en él y elige Liberar espacio.
        [many] { $count } de archivos descargados todavía ocupan { $size } en este portátil, porque su aplicación de la nube no los ha vuelto a dejar solo en línea. Para liberar el espacio, haz clic derecho en cada uno y elige Liberar espacio.
       *[other] { $count } archivos descargados todavía ocupan { $size } en este portátil, porque su aplicación de la nube no los ha vuelto a dejar solo en línea. Para liberar el espacio, haz clic derecho en cada uno y elige Liberar espacio.
    }
kept-online-paused-metered = La descarga se detuvo porque esta conexión puede cobrar por los datos. El resto sigue a salvo en línea.
kept-online-paused-space = La descarga se detuvo porque este portátil se está quedando sin espacio. El resto sigue a salvo en línea.
plan-own-folder = Poner las cosas de { $laptop } en su propia carpeta
plan-own-folder-help = Este portátil ya tiene cosas de otro portátil antiguo. Una carpeta para cada portátil las mantiene separadas.
plan-own-folder-name = De { $laptop }
plan-own-folder-name-unnamed = Del otro portátil
plan-downloads =
    { NUMBER($count) ->
        [one] Se descarga 1 aplicación de internet para instalarla en este portátil.
        [many] Se descargan { $count } de aplicaciones de internet para instalarlas en este portátil.
       *[other] Se descargan { $count } aplicaciones de internet para instalarlas en este portátil.
    }
plan-wait-free-wifi = Esperar a una Wi-Fi gratuita antes de descargar
plan-wait-free-wifi-help = Esta conexión puede cobrar por los datos. Con esto activado, PCTwin espera hasta 20 minutos a una gratuita. Las aplicaciones que no pueda descargar aparecen en la lista para que las instales más tarde.
plan-reuse-installers =
    { NUMBER($count) ->
        [one] Usar el instalador encontrado en el portátil antiguo para 1 aplicación, en lugar de descargarla
        [many] Usar los instaladores encontrados en el portátil antiguo para { $count } de aplicaciones, en lugar de descargarlas
       *[other] Usar los instaladores encontrados en el portátil antiguo para { $count } aplicaciones, en lugar de descargarlas
    }
plan-reuse-installers-help = Cada uno se comprueba primero, igual que una descarga: debe estar firmado por el propio fabricante de la aplicación.
