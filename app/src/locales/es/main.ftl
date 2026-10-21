# PCTwin screens, Spanish (es). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = Inicio de PCTwin
language-label = Idioma

footer-free = Traslados personales gratis. Sin cuenta.
footer-help = Ayuda

## F01 Welcome

welcome-eyebrow = Tu próximo capítulo, listo.
welcome-title-line1 = Tu configuración.
welcome-title-line2 = Tu próximo portátil.
welcome-body = Llévate tus archivos, aplicaciones y ajustes de siempre en el traslado.
welcome-start = Empezar un traslado

## F06 Old or new laptop

role-back = Atrás
role-eyebrow = Empezar un traslado
role-title = ¿Qué portátil es este?
role-body = Abre PCTwin en los dos portátiles. Luego indica cuál es este.
role-old-title = Mi portátil antiguo
role-old-body = Enviar mis archivos, aplicaciones y ajustes.
role-new-title = Mi portátil nuevo
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
check-speed-unknown = PCTwin no pudo medir la conexión esta vez. Vuelve a comprobarla en un momento.
check-space-unknown = PCTwin no pudo leer el espacio libre del portátil nuevo.
check-power-unknown = PCTwin no pudo saber si { $label } está enchufado.
check-internet-unknown = PCTwin no pudo saber si este portátil está en línea. No pasa nada: copiar archivos no necesita internet.
check-again = Comprobar de nuevo
check-estimate-note = El tiempo calculado es aproximado, no una promesa.
check-continue = Elegir qué trasladar

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

## C03 Old laptop: show the code

pair-old-eyebrow = Portátil antiguo
pair-old-title = Conectemos tus portátiles.
pair-old-body = En tu portátil nuevo, escribe el código de abajo.
pair-old-next = Después, elegirás un número que aparece en los dos portátiles.
pair-old-network = Deja los dos portátiles en la misma Wi-Fi.
pair-old-waiting = Esperando a tu portátil nuevo…
pair-old-starting = Preparando un código…
pair-old-label-title = Nombre de este portátil por ahora
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
pair-old-locked-title = La vinculación se detuvo.
pair-old-locked-body = Hubo demasiados intentos fallidos, así que la vinculación se detuvo para proteger este portátil. No se compartió nada. Empieza de nuevo cuando quieras.
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
pair-new-list-title = ¿Cuál es tu portátil antiguo?
pair-new-list-body = Elige el nombre que muestra la pantalla de tu portátil antiguo.
pair-new-same-label = Otro portátil cercano tiene este nombre. Mira la pantalla del portátil antiguo.
pair-new-none-title = No aparece tu portátil antiguo.
pair-new-none-body = Comprueba que PCTwin esté abierto en tu portátil antiguo y muestre un código, y que los dos portátiles estén en la misma Wi-Fi. Luego busca de nuevo.
pair-new-search-again = Buscar de nuevo
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
pair-done-body = Tus dos portátiles están conectados. Lo siguiente es elegir qué trasladar, y esa parte aún se está creando, así que todavía no se ha trasladado nada.

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
pairing-failed = La vinculación no funcionó esta vez. No se compartió nada. Inténtalo de nuevo.

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
help-contains-versions = La versión de PCTwin y el sistema de cada portátil, como Windows o macOS.
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
plan-version = Versión del plan { $revision }
plan-details = Más detalles
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

## X01 Move progress, compact

progress-eyebrow = Tu traslado
progress-title = Preparando tu nuevo portátil.
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
progress-low-battery = A { $label } le queda poca batería. Enchúfalo para que el traslado no se interrumpa.
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
outcome-files-title = Archivos seleccionados
outcome-status-partial = Hecho en parte
outcome-next-title = Qué hacer ahora
outcome-old-safe = Lo que no terminó sigue en esta lista. No se quitó nada de tu portátil antiguo.
outcome-report = Ver el informe completo
outcome-home = Volver al inicio

## R02 Your move, item by item (detailed report)

report-title = Tu traslado, elemento por elemento.
report-body = Qué pasó con cada elemento, por qué, de dónde vino y adónde fue.
report-filter-label = Mostrar
report-col-item = Elemento
report-col-result = Resultado
report-col-details = Comprobación o siguiente paso
report-from = De: { $path }
report-to = A: { $path }
report-none = Ningún elemento tiene este resultado.
report-more-details = Más detalles

# Evidence: how a copied item was checked.
evidence-read-back = Comprobado volviendo a leerlo del disco.
evidence-memory = Comprobado con la memoria de este portátil, no con el disco.
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
reason-sign-in = Ya está en su sitio, pero tienes que entrar con tu cuenta. Ábrela y entra con tu cuenta.
reason-activation = La aplicación ya está en su sitio, pero necesita su licencia en el portátil nuevo. Ábrela y actívala con tu licencia o tu cuenta.
reason-permission = Primero el portátil nuevo tiene que dar permiso. Abre el elemento y elige Permitir cuando te lo pida.
reason-online-only = Este archivo solo está guardado en línea, así que no estaba en el portátil antiguo para copiarlo. Descárgalo en el portátil nuevo desde la aplicación que lo guarda en línea.
reason-manual-install = Esta aplicación hay que conseguirla en la web de su fabricante. Descárgala e instálala desde allí.
reason-restart = El portátil nuevo tiene que reiniciarse para terminar. Reinícialo cuando te venga bien.
reason-finish-by-hand = PCTwin no puede hacer este paso por ti. Sigue los pasos que aparecen para él en este informe.

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
qs-eyebrow-receiving = Recibiendo en este portátil
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = De { $from } a { $to }
qs-route-sender = Envía · este portátil
qs-route-receiver = Recibe · misma Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } elemento seleccionado · { $size }
        [many] { $count } de elementos seleccionados · { $size }
       *[other] { $count } elementos seleccionados · { $size }
    }
qs-tally-badge = Envío cercano

## Q01 Quick Send home

qs-home-title = Unos pocos archivos. Directos al otro lado.
qs-home-body = Envía unos pocos archivos a un portátil cercano.
qs-send-title = Enviar archivos
qs-send-body = Elige archivos o carpetas de este portátil.
qs-receive-title = Recibir archivos
qs-receive-body = Deja que un portátil cercano envíe archivos a este.
qs-history-title = Tus recibos
qs-history-body = Mira adónde fueron los archivos recibidos y qué no terminó.
qs-home-action = Elegir archivos
qs-receiving = Listo para recibir. Los demás portátiles cercanos ven este como { $label }.
qs-receive-stop = Dejar de recibir
qs-no-nearby = No se encontró ningún portátil cerca. Abre PCTwin en el otro portátil y deja los dos en la misma Wi-Fi.

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
qs-online-only = Algunos archivos están solo en línea. Haz que estén disponibles en este portátil y luego envíalos.
qs-protected = PCTwin no puede leer { $name }. Permite el acceso o quítalo de la lista.
qs-allow = Permitir acceso
qs-choose-continue = Revisar estos archivos

## Q03 Check what you are sharing

qs-check-title = Revisa lo que vas a compartir.
qs-check-body = Mira bien estos archivos antes de que salgan de este portátil.
qs-check-included = Incluido
qs-check-shared-note = El otro portátil ve el nombre, el tamaño y la fecha de estos archivos.
qs-check-changed = { $name } cambió desde que lo elegiste. Revísalo de nuevo.
qs-check-ack = Ya revisé estos archivos
qs-check-ack-hint = Quita todo lo privado antes de enviar.
qs-check-continue = Elegir el otro portátil

## Q04 Who should receive it?

qs-who-title = ¿Quién debe recibirlo?
qs-who-body = Elige el nombre que aparece en el otro portátil.
qs-who-nearby = Cerca · misma Wi-Fi.
qs-who-code = Usar un código de vinculación
qs-who-code-body = Escribe el código que aparece en el portátil que envía.
qs-who-same-label = Dos portátiles cercanos tienen este nombre. Mira el nombre en la pantalla del otro portátil.
qs-who-busy = { $label } está ocupado con otro envío. Espera un momento e inténtalo de nuevo.
qs-who-note = El otro portátil acepta los archivos y elige adónde van.
qs-who-action = Mostrar mi código de envío

## Q08 Incoming offer (receiving laptop)

qs-offer-title = Alguien quiere enviarte archivos.
qs-offer-body = Revisa lo que te envían antes de aceptar.
qs-offer-note = PCTwin guarda solo los archivos que aceptas. Nunca los abre ni los pone en marcha.
qs-offer-accept = Aceptar y elegir una carpeta
qs-offer-decline = Rechazar estos archivos
qs-offer-declined = Los rechazaste. No se guardó nada en este portátil.
qs-offer-changed = Quien envía cambió los archivos. Revisa la lista nueva antes de aceptar.
qs-offer-gone = { $label } ya no está disponible. No se guardó nada en este portátil.

## Q09 Where should the files land?

qs-land-title = ¿Dónde se guardan los archivos?
qs-land-body = Elige la carpeta de este portátil.
qs-land-folder-label = Carpeta de destino
qs-land-choose = Elegir otra carpeta
qs-land-space-title = Espacio disponible
qs-land-space = { $free } disponibles · { $needed } necesarios.
qs-land-status-fits = Cabe
qs-land-status-no-space = No hay espacio suficiente
qs-land-no-space = Esta carpeta tiene { $free } libres, pero estos archivos necesitan { $needed }. Elige otra carpeta o libera espacio.
qs-land-unavailable = Esta carpeta no está disponible. Conecta su disco o elige otra carpeta.
qs-land-denied = PCTwin no tiene permiso para guardar en esta carpeta. Elige otra carpeta.
qs-land-note = Si ya hay un archivo con el mismo nombre, tú eliges qué pasa antes de guardar nada.
qs-land-confirm = Usar esta carpeta para estos archivos
qs-land-confirm-hint = Aquí se guardan solo los archivos que aceptaste.
qs-land-use = Usar esta carpeta

## Q10 Keep your existing work safe

qs-conflict-title = Mantén a salvo tu trabajo.
qs-conflict-body = Elige qué pasa cuando ya hay un archivo con el mismo nombre.
qs-conflict-col-file = Elemento
qs-conflict-col-existing = Ya estaba
qs-conflict-col-choice = Opción
# $when is already formatted, for example "yesterday".
qs-conflict-existing = Editado { $when }
qs-conflict-none = No hay copia anterior
qs-conflict-select =
    .aria-label = Qué pasa con { $name }
qs-keep-both = Conservar ambos
qs-skip = Omitir este elemento
qs-add-new = Añadir nuevo
qs-keep-both-note = Conservar ambos deja como está el archivo que ya había. El nuevo se guarda como { $newName }.
qs-no-conflicts = Ninguno de estos archivos está ya en la carpeta. No se reemplazará nada.
qs-conflict-target-changed = Si el archivo que ya había cambia o no se puede abrir, espera hasta que lo revises de nuevo.
qs-conflict-approve = Apruebo estas opciones
qs-conflict-continue = Confirmar estas opciones

## Q11 Ready to send these files?

qs-approve-title = ¿Todo listo para enviar estos archivos?
qs-approve-body = Revisa exactamente lo que se envía antes de empezar.
qs-approve-receiver-title = Otro portátil
qs-approve-receiver-waiting = Esperando a que el otro portátil acepte y elija una carpeta.
qs-approve-receiver-ready = El otro portátil aceptó y eligió una carpeta y qué hacer con los archivos que ya había.
qs-approve-files-title = Archivos
qs-approve-files-same = No cambies estos archivos hasta que se envíen.
qs-approve-files-changed = Algunos archivos cambiaron desde que los revisaste. Revísalos de nuevo.
qs-approve-existing-title = Archivos que ya había
qs-approve-existing-body = Conservar ambos, según lo elegido.
qs-status-waiting = En espera
qs-status-accepted = Aprobado
qs-status-unchanged = Revisar
qs-status-changed = Cambiado
qs-approve-ack = Enviar estos archivos
qs-approve-ack-hint = Apruebo estos archivos y este portátil.
qs-approve-start = Empezar a enviar
qs-approve-cleared = Cambiaste los archivos, así que { $label } debe aceptarlos de nuevo.
qs-approve-withdrawn = { $label } ya no acepta este envío. No se envió nada.

## Q12 Send progress

qs-sending-title = Tus archivos están en camino.
qs-sending-body = Las copias van directas de un portátil al otro.
qs-sending-progress =
    .aria-label = Enviando los archivos seleccionados
qs-sending-current = Enviando: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, medida
qs-step-copying = Copiando
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } elemento seleccionado recibido.
        [many] { $done } de elementos seleccionados recibidos.
       *[other] { $done } elementos seleccionados recibidos.
    }
qs-step-checking = Comprobando
qs-step-checking-body = Cada archivo se compara con el original.
qs-step-receipt = Recibo
qs-step-receipt-body = Al final, mira adónde fueron los archivos y qué no terminó.
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
qs-sending-complete-body = Cada archivo que llegó coincidía con su original. Mira dónde se guardaron.
qs-sending-receipt = Ver el recibo

## Q14 Handover receipt (both laptops)

qs-receipt-title = Aquí tienes tu recibo.
qs-receipt-body = Mira exactamente qué llegó y dónde se guardó.
qs-receipt-box-title = Tus archivos llegaron
qs-receipt-box-body = Cada copia se comparó con el original y coincidía.
qs-receipt-arrived =
    { NUMBER($count) ->
        [one] { $count } elemento seleccionado recibido
        [many] { $count } de elementos seleccionados recibidos
       *[other] { $count } elementos seleccionados recibidos
    }
qs-receipt-size = { $size } coinciden con el original
qs-receipt-saved-as = { $path }
qs-receipt-not-arrived = No todo el contenido de esta carpeta llegó y se comprobó.
qs-receipt-status-verified = Comprobado
qs-receipt-status-unfinished = Sin terminar
qs-receipt-not-completed = No se completó.
qs-receipt-stopped-row = Recibido y comprobado antes de parar.
qs-receipt-partial =
    { NUMBER($count) ->
        [one] { $count } elemento aún necesita atención.
        [many] { $count } de elementos aún necesitan atención.
       *[other] { $count } elementos aún necesitan atención.
    }
qs-receipt-stopped = Este envío se paró. Los archivos que llegaron completos siguen en la carpeta que elegiste.
qs-receipt-not-opened = PCTwin no abrió ni puso en marcha ningún archivo recibido.
qs-receipt-ack = Confirmar el recibo
# $when is already formatted, for example "today at 14:05".
qs-receipt-acked = Recibo confirmado { $when }.
qs-receipt-ack-note = Confirmar este recibo no significa que los archivos se hayan abierto o usado.
qs-receipt-offline = { $label } no tiene conexión, así que aún no ha visto este recibo. El recibo está guardado en este portátil.
qs-receipt-missing = Enviar lo que falta
qs-receipt-missing-body = Revisa solo los archivos que no llegaron.
qs-receipt-history = Todos los recibos
qs-receipt-done = Volver a Envío rápido

## Q19 Finish the missing part

qs-missing-title = Termina la parte que falta.
qs-missing-body = Usa el recibo anterior para ver qué no llegó.
qs-missing-status-there = Llegó
qs-missing-status-missing = Requiere acción
qs-missing-there-body = Llegó y coincidía con el original.
qs-missing-missing-body = Revisa los archivos que faltan o que no terminaron.
qs-missing-note = Los archivos que llegaron se conservan. Los archivos nuevos o cambiados necesitan tu aprobación de nuevo.
qs-missing-ack = Enviar los archivos que faltan
qs-missing-ack-hint = PCTwin vuelve a revisar los archivos y el otro portátil los acepta de nuevo.
qs-missing-complete = Ya llegó todo. No queda nada por enviar.
qs-missing-unavailable = No se encuentra el recibo anterior, así que PCTwin no puede saber qué falta. Empieza un envío nuevo.
qs-missing-continue = Revisar lo que se enviará

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
qs-stop-confirm = Parar y ver el recibo
qs-stop-keep = Seguir enviando
qs-decline-title = ¿Rechazar estos archivos?
qs-decline-body = No se guarda nada en este portátil, y a quien envía se le avisa de que los rechazaste.
qs-decline-confirm = Rechazar
qs-already-finished = Este envío ya terminó. Mira el recibo para ver qué llegó.

## F08 Your moves, kept on this laptop (history)

history-title = Tus traslados, guardados en este portátil.
history-body = Abre de nuevo un informe o sigue con un traslado guardado.
history-row-move = { $from } → { $to }
history-row-sent = Enviado a { $label }
history-row-received = Recibido de { $label }
history-status-finished = Completado
history-status-steps-left =
    { NUMBER($count) ->
        [one] Completado, te queda { $count } paso
        [many] Completado, te quedan { $count } de pasos
       *[other] Completado, te quedan { $count } pasos
    }
history-status-paused = En pausa
history-status-stopped = Parado sin riesgo
history-status-partial = No llegó todo
history-open-report = Informe
history-open-receipt = Recibo
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
qs-check-missing = { $name } ya no está ahí. Vuelve atrás y quítalo, o vuelve a ponerlo en su sitio.
qs-check-unreadable = PCTwin ya no puede leer { $name }. Permite el acceso, o vuelve atrás y quítalo.
qs-check-show-all =
    { NUMBER($count) ->
        [one] Mostrar el archivo
        [many] Mostrar los { $count } de archivos
       *[other] Mostrar los { $count } archivos
    }
qs-check-change = Cambiar lo que envío
qs-land-none = Aún no has elegido carpeta.
qs-land-space-unknown = PCTwin no pudo medir el espacio libre aquí. Estos archivos necesitan { $needed }.
qs-conflict-existing-unknown = Ya estaba
qs-conflict-choice-open = Sin elegir
qs-conflict-why = Elige qué pasa con cada archivo y marca la casilla para continuar.
qs-approve-folder-title = Carpeta de destino
qs-status-recipient-choice = Lo elige quien recibe
qs-sending-card = Enviando los archivos seleccionados
qs-progress-of = de { $total }
qs-receipt-partial-title = No llegó todo
qs-receipt-metric-items =
    { NUMBER($count) ->
        [one] elemento seleccionado recibido
        [many] de elementos seleccionados recibidos
       *[other] elementos seleccionados recibidos
    }
qs-receipt-metric-size = comprobado con el original
qs-receipt-sent-as = Enviado como { $path }
qs-receipt-renamed = Ya había un archivo con ese nombre, así que este tiene un nombre nuevo.
qs-receipt-chip-already = Ya estaba
qs-receipt-already = Ya había un archivo idéntico ({ $path }), así que no se envió de nuevo.
qs-receipt-chip-declined = No aceptado
qs-receipt-declined-send = El otro portátil no aceptó este archivo, así que no se envió.
qs-receipt-declined-receive = No aceptaste este archivo, así que no se guardó.
qs-receipt-more = Más detalles
qs-receipt-empty = No se envió nada, así que no hay nada que mostrar.
qs-receipt-already-count =
    { NUMBER($count) ->
        [one] { $count } elemento ya estaba como archivo idéntico
        [many] { $count } de elementos ya estaban como archivos idénticos
       *[other] { $count } elementos ya estaban como archivos idénticos
    }
qs-missing-new-send = Empezar un envío nuevo
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
