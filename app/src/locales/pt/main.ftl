# PCTwin screens, Brazilian Portuguese (pt). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = Página inicial do PCTwin
language-label = Idioma

footer-free = Mudanças pessoais grátis. Não precisa de conta.
footer-help = Ajuda

## F01 Welcome

welcome-eyebrow = Seu próximo capítulo, pronto.
welcome-title-line1 = Sua configuração completa.
welcome-title-line2 = Seu próximo notebook.
welcome-body = Leve seus arquivos, apps e as configurações de sempre na mudança.
welcome-start = Começar uma mudança

## F06 Old or new laptop

role-back = Voltar
role-eyebrow = Começar uma mudança
role-title = Qual notebook é este?
role-body = Abra o PCTwin nos dois notebooks. Depois diga qual é este.
role-old-title = Meu notebook antigo
role-old-body = Enviar meus arquivos, apps e configurações.
role-new-title = Meu notebook novo
role-new-body = Receber minha configuração completa de outro notebook.
role-note = Nada é movido até você revisar e aprovar o plano. Seu notebook antigo continua como está durante a mudança.

## Frame parts (footer note, Back, steps)

footer-note-move = Mudanças pessoais grátis
footer-note-send = Envio grátis por perto
frame-back = Voltar
# Read out by screen readers only; the step shapes carry the meaning on screen.
frame-step-done = Concluída
frame-step-current = Agora
frame-step-todo = Próxima
frame-step-position = Etapa { $number } de { $total }

## F01 Unfinished move

resume-open = Retomar sua mudança

## C01 How will you connect?

connect-eyebrow = Sua mudança
connect-title = Como você vai conectar?
connect-body = Seja qual for sua escolha, as etapas são as mesmas.
connect-wifi-title = Wi-Fi
connect-wifi-body = Os dois notebooks no mesmo Wi-Fi.
connect-wifi-status = Disponível
connect-later = Cabo de rede, o cabo-ponte do PCTwin e a mudança por um disco externo estão previstos para uma versão futura.
connect-usb-note = Um cabo USB comum não consegue conectar dois notebooks para uma mudança.
connect-use-wifi = Usar o Wi-Fi
connect-unit-mbps = { NUMBER($value, maximumFractionDigits: 0) } MB/s
connect-unit-gb = { NUMBER($value, maximumFractionDigits: 1) } GB

## C02 Put both laptops together

connect-prep-title = Coloque os dois notebooks juntos.
connect-prep-body = Conecte os dois ao mesmo Wi-Fi e abra o PCTwin em cada um.
connect-prep-step-1-title = Entre no mesmo Wi-Fi
connect-prep-step-1-body = Copiar arquivos não precisa de internet.
# $network is the Wi-Fi name this laptop is on.
connect-prep-step-1-network = Rede: { $network }.
connect-prep-step-2-title = Permita encontrar notebooks por perto
connect-prep-step-2-body = Se o Windows ou o macOS pedir permissão, escolha Permitir.
connect-prep-step-3-title = Escolha uma função em cada notebook
connect-prep-step-3-body = Um envia. O outro recebe.
connect-prep-note = Um Wi-Fi de visitante ou público pode impedir que os notebooks se vejam.
connect-prep-continue = Ir para o pareamento

## C06 Meet your other laptop

peer-eyebrow = Sua mudança
peer-title = Conheça seu outro notebook.
peer-body = Os dois notebooks estão conectados e prontos.
peer-sending = Enviando
peer-receiving = Recebendo
peer-old = Notebook antigo
peer-new = Notebook novo
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
peer-status-update = Precisa atualizar
peer-status-unsupported = Esperando
peer-ready-note = O PCTwin só lista seus arquivos depois que os dois notebooks estiverem pareados. Se as versões forem diferentes, atualize o PCTwin nos dois.
peer-version-differs = Estes notebooks têm versões diferentes do PCTwin. Atualize o PCTwin nos dois para continuar.
peer-version-old-body = Versão diferente: { $version }.
peer-version-new-body = Precisa da mesma versão nos dois notebooks.
peer-update-title = Atualizar o PCTwin
peer-update-body = Instale a mesma versão nos dois notebooks.
# $from and $to are system names, for example "Windows 11" and "macOS". The mockup sentence does not use them.
peer-direction-unsupported = Esta versão do PCTwin ainda não faz mudanças entre estes dois sistemas.
peer-continue = Verificar a conexão

## C07 Let’s check the connection

check-eyebrow = Sua mudança
check-title = Vamos verificar a conexão.
check-body = O PCTwin verifica a velocidade, o espaço e a energia para planejar a mudança e estimar o tempo.
check-measuring = Medindo a conexão…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = velocidade da conexão
check-space-label = livre no notebook novo
check-link-weak = Fraca, então a mudança pode ser lenta. Aproximar os notebooks do roteador Wi-Fi pode ajudar.
check-power-title = Energia
check-power-both = Os dois notebooks na tomada.
check-power-battery = { $label } está usando a bateria. Ligue o notebook na tomada para que uma mudança longa não seja interrompida.
check-space-title = Armazenamento
check-space-body = { $free } livres no notebook novo. Antes de qualquer coisa ser movida, você vai ver se suas escolhas cabem.
check-internet-title = Internet
check-internet-on = Conectado. Copiar arquivos não precisa de internet.
check-internet-off = Sem conexão. Tudo bem: copiar arquivos não precisa de internet.
check-status-ready = Pronto
check-status-plug-in = Ligar na tomada
check-status-connected = Disponível
check-status-not-needed = Não precisa
# Added: what is said when something could not be read, so it is never shown as fine.
check-status-unknown = Desconhecido
check-speed-unknown = O PCTwin não conseguiu medir a conexão desta vez. Verifique de novo daqui a pouco.
check-space-unknown = O PCTwin não conseguiu ler o espaço livre no notebook novo.
check-power-unknown = O PCTwin não conseguiu saber se { $label } está ligado na tomada.
check-internet-unknown = O PCTwin não conseguiu saber se este notebook está online. Tudo bem: copiar arquivos não precisa de internet.
check-again = Verificar de novo
check-estimate-note = A estimativa de tempo não é uma promessa fixa.
check-continue = Escolher o que mover

## C09 Waiting for your other laptop

reconnect-title = Esperando seu outro notebook.
reconnect-body = Tudo o que já foi copiado e verificado é mantido.
# Added: the title of the saved-progress card (the mockup shows Move paused by connection).
reconnect-card-title = Mudança pausada pela conexão
reconnect-progress =
    .aria-label = Mudança pausada pela conexão
# $done is an already formatted size, for example "81.6 GB". $total is passed but not shown.
reconnect-saved = { $done } copiados e verificados até agora. O resto ainda vai chegar.
# $ago is already formatted, for example "45 seconds ago".
reconnect-last-seen = Visto pela última vez { $ago }.
reconnect-status-offline = Offline
reconnect-status-trying = Reconectando
reconnect-status-pending = Esperando
reconnect-trying = Tentando reconectar automaticamente… Mantenha os dois notebooks ligados, ativos e no mesmo Wi-Fi.
reconnect-expired = Passou tempo demais para reconectar automaticamente. Pareie de novo, e o PCTwin verifica o que já foi copiado.
reconnect-unavailable = Não foi possível alcançar { $label }. Verifique se ele está ligado, ativo, com o PCTwin aberto e no mesmo Wi-Fi.
reconnect-try-now = Tentar conectar de novo
reconnect-pair-again = Parear de novo
reconnect-details = Reconectar com segurança
reconnect-details-body = Só estes dois notebooks podem se reconectar, e a cada vez eles usam uma nova conexão privada.

## C10 The connection has ended (dialog)

ended-title = A conexão terminou.
ended-body = Sua mudança está salva. O código antigo não funciona mais.
ended-kept-title = Mantido
ended-kept-body = O registro da sua mudança e tudo o que foi copiado e verificado.
ended-not-kept-title = Não mantido
ended-not-kept-body = A permissão para continuar sem parear de novo.
ended-pair-again = Parear de novo
ended-pair-again-body = Conecte de novo, e o PCTwin verifica o que já foi copiado.
ended-see-moved = Abrir a mudança salva
ended-see-moved-body = Veja o que já foi copiado e verificado até agora.
ended-home = Voltar ao início

## Pairing, shared

pair-back = Voltar
pair-try-again = Tentar de novo
pair-more-details = Mais detalhes
pair-heads-up-title = Seu notebook pode fazer uma pergunta.
pair-heads-up-local-network = Seu Mac vai perguntar se o PCTwin pode encontrar dispositivos na sua rede local. Escolha Permitir para que os dois notebooks possam se encontrar.
pair-heads-up-firewall = O Windows pode perguntar se o PCTwin pode aceitar conexões. Escolha Permitir para que o notebook novo consiga chegar a este.
pair-heads-up-continue = Continuar

## C03 Old laptop: show the code

pair-old-eyebrow = Notebook antigo
pair-old-title = Vamos conectar seus notebooks.
pair-old-body = No seu notebook novo, digite o código abaixo.
pair-old-next = Depois, você vai confirmar um número que aparece nos dois notebooks.
pair-old-network = Mantenha os dois notebooks no mesmo Wi-Fi.
pair-old-waiting = Esperando seu notebook novo…
pair-old-starting = Preparando um código…
pair-old-label-title = O nome deste notebook por enquanto
pair-old-label-hint = Seu notebook novo vai encontrar este por esse nome.
pair-old-shuffle = Escolher outro nome
pair-old-rename = Digitar um nome
pair-old-rename-label = Nome para este notebook
pair-old-rename-note = Qualquer pessoa neste Wi-Fi pode ver esse nome.
pair-old-rename-save = Usar este nome
pair-old-rename-cancel = Cancelar
pair-old-code-meta =
    { $seconds ->
        [one] Muda a cada minuto. Novo código em { $seconds } segundo.
        [many] Muda a cada minuto. Novo código em { $seconds } de segundos.
       *[other] Muda a cada minuto. Novo código em { $seconds } segundos.
    }
pair-old-steps-title = No seu notebook novo
pair-old-step-1 = Abra o PCTwin e escolha Meu notebook novo.
pair-old-step-2 = Se aparecer uma lista, escolha o nome deste notebook.
pair-old-step-3 = Digite o código de seis dígitos mostrado aqui.
pair-old-wait-title = Um novo código está a caminho.
pair-old-wait-body =
    { $seconds ->
        [one] Essa tentativa não deu certo. Um novo código aparece em { $seconds } segundo.
        [many] Essa tentativa não deu certo. Um novo código aparece em { $seconds } de segundos.
       *[other] Essa tentativa não deu certo. Um novo código aparece em { $seconds } segundos.
    }
pair-old-locked-title = Alguém nesta rede tentou códigos errados.
pair-old-locked-body = Depois de cinco códigos errados, o pareamento parou para manter este notebook seguro. Nada foi compartilhado. Se foi você, escolha Começar de novo para receber um código novo.
pair-old-locked-hotspot = Se você não sabe quem foi, conecte os dois notebooks ao ponto de acesso do seu celular e depois escolha Começar de novo.
pair-old-start-again = Começar de novo
pair-old-timed-out = Nenhum número foi escolhido a tempo, então aqui está um novo código.
pair-old-declined = Os números não combinaram, então nada foi compartilhado. Aqui está um novo código.
pair-old-left = O notebook novo parou o pareamento antes de um número ser escolhido, então nada foi compartilhado. Aqui está um novo código.
pair-network-title = Escolha seu Wi-Fi.
pair-network-hint = Escolha a rede em que seu notebook novo está.

## C05 Old laptop: match the number

pair-choose-eyebrow = Confirme a conexão
pair-choose-title = Escolha o número igual.
pair-choose-body = Olhe para o seu notebook novo. Escolha o número que ele mostra.
pair-choose-number = Número { $number }
pair-choose-confirm = Confirmar estes notebooks
pair-choose-renewed = Novos números na tela. Escolha o que aparece no seu notebook novo.
pair-choose-none = Nenhum é igual
pair-choose-note = Se o notebook novo não mostrar nenhum número, ou se nenhum destes for igual, escolha Nenhum é igual.
pair-choose-safe = Nada foi compartilhado ainda.
pair-choose-checking = Verificando…

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = Notebook novo
pair-new-searching-title = Procurando seu notebook antigo…
pair-new-searching-body = Mantenha o PCTwin aberto no seu notebook antigo, no mesmo Wi-Fi que este.
pair-new-search-failed-title = Não foi possível procurar seu notebook antigo.
pair-new-list-title = Qual é o seu notebook antigo?
pair-new-list-body = Escolha o nome que seu notebook antigo mostra na tela.
pair-new-same-label = Outro notebook por perto tem este nome. Confira a tela do notebook antigo.
pair-new-needs-update = Este notebook tem uma versão mais antiga do PCTwin. Atualize o PCTwin nele e procure de novo.
pair-new-none-title = Nenhum notebook antigo encontrado.
pair-new-none-body = Verifique se o PCTwin está aberto no seu notebook antigo mostrando um código e se os dois notebooks estão no mesmo Wi-Fi. Depois procure de novo.
pair-new-search-again = Procurar de novo
pair-new-code-title = Digite o código.
pair-new-code-body = Digite o código mostrado em { $label }.
pair-new-code-label = Código de seis dígitos
pair-new-code-hint = Veja na tela do seu notebook antigo. Ele muda a cada minuto.
pair-new-connect = Conectar
pair-new-connecting = Conectando…
pair-new-other-laptop = Escolher outro notebook
pair-new-number-eyebrow = Confirme a conexão
pair-new-number-title = Escolha este número no seu notebook antigo.
pair-new-number-tile = Seu número
pair-new-number-label = Seu número é { $number }
pair-new-number-body = Seu notebook antigo mostra três números. Escolha este lá.
pair-new-number-waiting = Esperando seu notebook antigo…

## Paired (both laptops)

pair-done-eyebrow = Tudo certo
pair-done-title = Seus notebooks estão pareados.
pair-done-body = Seus dois notebooks estão conectados. Agora o notebook antigo procura seus arquivos, e você aprova no novo o que será movido. Nada é movido antes da sua aprovação.
pair-done-continue = Continuar

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = O código precisa ter seis dígitos. Confira a tela do notebook antigo e digite de novo.
pairing-code-wrong = Esse código não confere. Confira a tela do notebook antigo e tente de novo.
pairing-code-expired = Esse código mudou. Digite o novo que está na tela do notebook antigo.
pairing-busy = O notebook antigo já está pareando com outro notebook. Espere um pouco e tente de novo.
pairing-wait = O notebook antigo está preparando um novo código. Espere e depois digite o novo código.
pairing-locked = O pareamento parou depois de tentativas erradas demais. No notebook antigo, escolha Começar de novo.
pairing-unreachable = Este notebook não conseguiu alcançar o notebook antigo. Verifique se os dois estão no mesmo Wi-Fi e tente de novo.
pairing-wrong-number = Esse número não é igual ao do notebook novo. Nada foi compartilhado. Digite o novo código no notebook novo para tentar de novo.
pairing-no-match = Os números não combinaram no notebook antigo. Nada foi compartilhado. Digite o novo código que está na tela do notebook antigo.
pairing-timed-out = Nenhum número foi escolhido a tempo. Tente de novo com o novo código do notebook antigo.
pairing-connection-lost = A conexão entre os notebooks caiu. Nada foi compartilhado. Tente de novo.
pairing-connection-lost-new = A conexão com o notebook antigo caiu. Nada foi compartilhado.
pairing-different-version = Os dois notebooks têm versões diferentes do PCTwin. Atualize o PCTwin nos dois e tente de novo.
pairing-name-invalid = Esse nome não pode ser usado. Tente um nome mais curto, com letras e números.
pairing-no-network = Este notebook não está em uma rede Wi-Fi ou com cabo. Conecte o notebook e tente de novo.
pairing-choose-network = Este notebook está em mais de uma rede. Escolha aquela em que seu notebook novo está:
pairing-local-network-refused = Talvez seu Mac esteja impedindo o PCTwin de encontrar notebooks nesta rede. Se o Mac acabou de perguntar, escolha Permitir e tente de novo. Se não, abra Ajustes do Sistema, escolha Privacidade e Segurança e depois Rede Local, ative o PCTwin e tente de novo.
pairing-local-network-refused-details = Uma VPN, um firewall ou um app de filtro também pode impedir que os notebooks se encontrem. Se algum estiver ligado, desligue-o durante o pareamento e tente de novo.
pairing-failed = O pareamento não deu certo desta vez. Nada foi compartilhado. Tente de novo.
pairing-failed-new = O pareamento não funcionou desta vez. Nada foi compartilhado. Digite o novo código da tela do notebook antigo.

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = Sua mudança
people-title = Associe cada pessoa à conta dela.
people-body = Escolha para onde vão os arquivos de cada pessoa no notebook novo.
people-col-old = Notebook antigo
people-col-new = Notebook novo
people-col-status = Situação
people-you = Você
# $name is the person's name on the old laptop. $account is an account name on the new laptop.
people-select =
    .aria-label = Conta no notebook novo para { $name }
people-option-existing = { $account } / notebook novo
people-option-new = Criar uma conta nova
people-option-leave-out = Deixar de fora
people-option-choose = Escolher…
people-new-account-hint = O notebook novo pede permissão antes de criar uma conta.
people-suggested = Sugerido porque os nomes são iguais. Confira se é a mesma pessoa.
# $account is an account name on the new laptop.
people-suggested-use = Usar { $account }
people-status-ready = Confirmado
people-status-waiting = Precisa da aprovação da pessoa
people-status-left-out = Fora da mudança
people-status-choose = Precisa da sua escolha
people-status-changed = Alterado por você
people-waiting-body = { $name } precisa entrar na conta no notebook antigo e aprovar antes que os arquivos dessa pessoa possam ser movidos. Até lá, os arquivos ficam intactos.
people-just-me = Só a sua conta foi encontrada no notebook antigo.
people-changed-note = Você mudou para onde vão os arquivos de alguém. Confira antes de continuar.
people-shared-title = Itens compartilhados
people-shared-body = Pastas compartilhadas e documentos públicos que pertencem a todos no notebook antigo.
people-shared-select =
    .aria-label = Para onde vão os itens compartilhados
people-shared-option-shared = Pastas compartilhadas no notebook novo
people-shared-option-person = Com os arquivos de { $name }
people-shared-option-leave-out = Deixar de fora
people-note = O PCTwin nunca muda a senha de ninguém nem contorna o login de outra pessoa.
people-list-label = Pessoas no notebook antigo
people-todo =
    { $count ->
        [0] Todas as pessoas que podem ser incluídas já têm um destino.
        [one] { $count } pessoa ainda precisa de uma escolha.
        [many] { $count } de pessoas ainda precisam de uma escolha.
       *[other] { $count } pessoas ainda precisam de uma escolha.
    }
people-waiting-count =
    { $count ->
        [one] { $count } pessoa ainda precisa dar a aprovação. Você pode continuar; os arquivos dela ficam intactos até ela aprovar.
        [many] { $count } de pessoas ainda precisam dar a aprovação. Você pode continuar; os arquivos delas ficam intactos até elas aprovarem.
       *[other] { $count } pessoas ainda precisam dar a aprovação. Você pode continuar; os arquivos delas ficam intactos até elas aprovarem.
    }
people-continue = Continuar

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = Sua mudança
before-title = Antes de deixar este notebook.
before-body = Confira estes itens enquanto ainda tem seu notebook antigo.
before-licences-title = Licenças de apps
before-licences-body = Em alguns apps, é você quem precisa mover a licença.
before-passwords-title = Recuperação de contas
before-passwords-body = Garanta que você ainda consegue entrar nas suas contas e em qualquer disco bloqueado.
before-codes-title = Tenha seus códigos de login à mão
before-codes-body = Se uma conta envia um código ou usa um app para confirmar que é você, mantenha esse celular ou app por perto.
before-online-title = Trabalho não salvo
before-online-body = Salve os rascunhos e confira se os arquivos guardados online estão atualizados.
before-done =
    .aria-label = Já fiz isto: { $item }
before-note = Não digite senhas nem chaves de recuperação aqui. Terminar esta lista não significa que já é seguro apagar o notebook antigo.
before-empty = Não é preciso fazer nada no seu notebook antigo antes da mudança.
before-continue = Voltar ao plano da mudança

## S11 Get help with this task (a short problem code)

help-eyebrow = Ajuda e privacidade
help-title = Peça ajuda com esta tarefa.
help-body = Compartilhe um código curto que mostra a alguém o que deu errado. Antes, confira o que ele inclui.
help-making = Criando um código…
help-code-title = Código do problema
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = Código do problema { $code }
help-contains-title = O que ele inclui
help-contains-problem = O tipo de problema e até onde a tarefa chegou.
help-contains-step = A etapa da mudança em que aconteceu.
help-contains-versions = A versão do PCTwin e o sistema deste notebook, como Windows ou macOS.
help-never-title = O que ele nunca inclui
help-chip-included = Incluído
help-chip-excluded = Nunca incluído
help-never-names = Seu nome ou o de qualquer outra pessoa.
help-never-files = O conteúdo dos seus arquivos ou qualquer senha.
help-never-network = Nomes de arquivos ou pastas, onde seus arquivos ficam ou nomes de Wi-Fi.
help-note = O código não dá a ninguém acesso aos seus notebooks.
help-copy = Copiar código
help-copied = Código copiado.
help-share = Compartilhar
help-share-hint = Envie pelo WhatsApp ou por qualquer app.
help-share-text = Meu código de problema do PCTwin é { $code }.
help-failed = Não foi possível criar um código de problema. Tente de novo. Se ainda não funcionar, descreva o problema com suas palavras.
help-try-again = Tentar de novo
help-copy-failed = Não foi possível copiar o código. Selecione o código acima e copie manualmente.

## Label words (the name each laptop shows while pairing)

label-colour-red =
    { $gender ->
        [feminine] vermelha
       *[masculine] vermelho
    }
label-colour-orange = laranja
label-colour-yellow =
    { $gender ->
        [feminine] amarela
       *[masculine] amarelo
    }
label-colour-green = verde
label-colour-blue = azul
label-colour-purple =
    { $gender ->
        [feminine] roxa
       *[masculine] roxo
    }
label-colour-pink = rosa
label-colour-brown = marrom
label-colour-black =
    { $gender ->
        [feminine] preta
       *[masculine] preto
    }
label-colour-white =
    { $gender ->
        [feminine] branca
       *[masculine] branco
    }
label-colour-grey = cinza
label-colour-gold =
    { $gender ->
        [feminine] dourada
       *[masculine] dourado
    }

label-animal-ant = Formiga
label-animal-badger = Texugo
label-animal-bat = Morcego
label-animal-bear = Urso
label-animal-bee = Abelha
label-animal-beetle = Besouro
label-animal-bison = Bisão
label-animal-butterfly = Borboleta
label-animal-camel = Camelo
label-animal-cat = Gato
label-animal-cheetah = Guepardo
label-animal-chicken = Galinha
label-animal-crab = Caranguejo
label-animal-crane = Grou
label-animal-crocodile = Crocodilo
label-animal-crow = Corvo
label-animal-deer = Cervo
label-animal-dolphin = Golfinho
label-animal-donkey = Burro
label-animal-dove = Pomba
label-animal-dragonfly = Libélula
label-animal-duck = Pato
label-animal-eagle = Águia
label-animal-elephant = Elefante
label-animal-falcon = Falcão
label-animal-flamingo = Flamingo
label-animal-fox = Raposa
label-animal-frog = Sapo
label-animal-gazelle = Gazela
label-animal-swordfish = Peixe-espada
label-animal-giraffe = Girafa
label-animal-goat = Cabra
label-animal-goose = Ganso
label-animal-gorilla = Gorila
label-animal-hamster = Hamster
label-animal-hawk = Gavião
label-animal-hedgehog = Ouriço
label-animal-armadillo = Tatu
label-animal-hippo = Hipopótamo
label-animal-horse = Cavalo
label-animal-hummingbird = Beija-flor
label-animal-jaguar = Jaguar
label-animal-jellyfish = Água-viva
label-animal-kangaroo = Canguru
label-animal-kingfisher = Martim-pescador
label-animal-koala = Coala
label-animal-ladybird = Joaninha
label-animal-lemur = Lêmure
label-animal-leopard = Leopardo
label-animal-lion = Leão
label-animal-lizard = Lagarto
label-animal-llama = Lhama
label-animal-lobster = Lagosta
label-animal-lynx = Lince
label-animal-macaw = Arara
label-animal-meerkat = Suricato
label-animal-mole = Toupeira
label-animal-moose = Alce
label-animal-pufferfish = Baiacu
label-animal-mouse = Rato
label-animal-octopus = Polvo
label-animal-ostrich = Avestruz
label-animal-otter = Lontra
label-animal-owl = Coruja
label-animal-panda = Panda
label-animal-kiwi = Quivi
label-animal-parrot = Papagaio
label-animal-peacock = Pavão
label-animal-pelican = Pelicano
label-animal-penguin = Pinguim
label-animal-puffin = Papagaio-do-mar
label-animal-rabbit = Coelho
label-animal-raccoon = Guaxinim
label-animal-rhino = Rinoceronte
label-animal-robin = Sabiá
label-animal-salmon = Salmão
label-animal-seahorse = Cavalo-marinho
label-animal-seal = Foca
label-animal-shark = Tubarão
label-animal-sheep = Ovelha
label-animal-sloth = Preguiça
label-animal-snail = Caracol
label-animal-sparrow = Pardal
label-animal-squirrel = Esquilo
label-animal-starfish = Estrela-do-mar
label-animal-stork = Cegonha
label-animal-swan = Cisne
label-animal-tiger = Tigre
label-animal-snake = Cobra
label-animal-toucan = Tucano
label-animal-turtle = Tartaruga
label-animal-walrus = Morsa
label-animal-whale = Baleia
label-animal-wolf = Lobo
label-animal-woodpecker = Pica-pau
label-animal-yak = Iaque
label-animal-zebra = Zebra
label-animal-dog = Cachorro
label-animal-cow = Vaca
label-animal-beaver = Castor

# Grammatical gender of each animal noun above (value is exactly masculine or feminine; not translated).
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
label-gender-crane = masculine
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
label-gender-fox = feminine
label-gender-frog = masculine
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
label-gender-macaw = feminine
label-gender-meerkat = masculine
label-gender-mole = feminine
label-gender-moose = masculine
label-gender-pufferfish = masculine
label-gender-mouse = masculine
label-gender-octopus = masculine
label-gender-ostrich = masculine
label-gender-otter = feminine
label-gender-owl = feminine
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
label-gender-sloth = feminine
label-gender-snail = masculine
label-gender-sparrow = masculine
label-gender-squirrel = masculine
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

# Translators: the laptop's name is a colour plus an animal, for example "Blue Fox".
# Word order, gender and agreement are yours to choose for your language.
label-picked = { $animal } { $colour }
label-unnamed = Notebook sem nome

## M02 Finding your files

scan-eyebrow = Sua mudança
scan-title = Encontrando sua configuração de sempre.
scan-body = O que o PCTwin encontra fica salvo, então a busca pode continuar se for interrompida.
scan-progress =
    .aria-label = Procurando nos lugares que você escolheu
# $size is passed but not shown in the mockup line.
scan-found =
    { NUMBER($count) ->
        [one] { $count } arquivo encontrado até agora.
        [many] { $count } de arquivos encontrados até agora.
       *[other] { $count } arquivos encontrados até agora.
    }
scan-step-usual-title = Arquivos e pastas
scan-step-usual-body = Busca feita nos lugares que o PCTwin pode ler.
scan-step-other-title = Outras pastas e discos
scan-step-other-body = Pastas e discos que você usa fora dos lugares de sempre.
scan-step-protected-title = Lugares protegidos
scan-step-protected-body =
    { NUMBER($count) ->
        [one] { $count } lugar ainda precisa da sua permissão.
        [many] { $count } de lugares ainda precisam da sua permissão.
       *[other] { $count } lugares ainda precisam da sua permissão.
    }
scan-step-done = Concluída
scan-step-current = Agora
scan-step-todo = Próxima
scan-unreadable-note = Os lugares que o PCTwin não conseguiu ler continuam na lista. Eles não contam como lugares já procurados.
scan-status-not-searched = Não procurado
scan-paused = Busca pausada. Tudo o que foi encontrado até agora está salvo.
scan-resumed = A busca continuou de onde parou.
scan-partial =
    { NUMBER($count) ->
        [one] { $count } lugar protegido ainda não foi procurado.
        [many] { $count } de lugares protegidos ainda não foram procurados.
       *[other] { $count } lugares protegidos ainda não foram procurados.
    }
scan-allow = Permitir acesso
scan-pause = Pausar
scan-resume = Continuar a busca
scan-continue = Ver o que foi encontrado

## M03 Everything you chose, in view

overview-eyebrow = Sua mudança
overview-title = Tudo o que você escolheu, à vista.
overview-body = Confira o que o PCTwin encontrou antes que ele faça o plano.
# The metric value next to each label is passed in already formatted.
overview-size-label = arquivos selecionados
overview-items-label =
    { NUMBER($count) ->
        [one] arquivo
        [many] de arquivos
       *[other] arquivos
    }
overview-attention-label =
    { NUMBER($count) ->
        [one] precisa da sua atenção
        [many] precisam da sua atenção
       *[other] precisam da sua atenção
    }
overview-files-title = Arquivos e pastas
overview-files-body = Documentos, fotos e outras pastas que você escolheu.
overview-dest-title = Para onde vão
overview-dest-body = O lugar no notebook novo para cada pasta.
overview-people-title = Pessoas
overview-people-body = Escolha a conta no notebook novo para cada pessoa.
overview-review = Revisar
overview-partial =
    { NUMBER($count) ->
        [one] { $count } lugar protegido ainda não foi procurado.
        [many] { $count } de lugares protegidos ainda não foram procurados.
       *[other] { $count } lugares protegidos ainda não foram procurados.
    }
overview-allow = Permitir acesso
overview-continue = Continuar

## M04 Keep what matters to you

files-eyebrow = Sua mudança
files-title = Mantenha o que importa para você.
files-body = Escolha pastas, outros discos e quaisquer outros lugares.
files-search-label = Encontrar uma pasta
files-search-placeholder = Procurar pastas
files-select =
    .aria-label = Mover { $name }
files-folder-detail =
    { NUMBER($count) ->
        [one] { $size } · { $count } arquivo
        [many] { $size } · { $count } de arquivos
       *[other] { $size } · { $count } arquivos
    }
files-status-protected = Precisa de permissão
files-protected-body = O PCTwin precisa da sua permissão para ler esta pasta. Os arquivos dela só contam depois que você permitir.
files-allow = Permitir acesso
files-status-added = Adicionada por você
files-add = Adicionar outra pasta
files-remove =
    .aria-label = Remover { $name }
files-no-limit = Mudanças grátis não têm limite de tamanho de arquivo nem de quantas pastas você move.
files-total =
    { NUMBER($count) ->
        [one] { $count } pasta escolhida, { $size } no total
        [many] { $count } de pastas escolhidas, { $size } no total
       *[other] { $count } pastas escolhidas, { $size } no total
    }
files-none = Escolha pelo menos uma pasta para continuar.
files-continue = Escolher destinos

## M05 Give every file a home

dest-eyebrow = Sua mudança
dest-title = Dê um lugar a cada arquivo.
dest-body = Escolha para onde vai cada pasta no notebook novo.
dest-col-chosen = O que você escolheu
dest-col-place = Para onde vai
dest-col-placement = Disco
dest-placement-local = Interno
dest-placement-external = Externo
dest-col-status = Situação
dest-select =
    .aria-label = Destino de { $name }
# $folder is a folder name on the new laptop, for example "Documents".
dest-option-folder = Notebook novo / { $folder }
# $drive is the name of a drive connected to the new laptop.
dest-option-drive = Disco externo / { $drive }
dest-option-choose = Escolher pasta
dest-default-label = Lugar de sempre
dest-status-ready = Pronto
dest-status-missing = Disco não conectado
dest-status-space = Espaço insuficiente
dest-all-ready = Cada pasta tem um lugar no notebook novo.
dest-missing = { $drive } não está conectado ao notebook novo. Conecte o disco ou escolha outro lugar para estes arquivos.
dest-space = { $place } tem { $free } livres, mas estes arquivos precisam de { $needed }. Libere espaço lá, escolha outro lugar ou mova menos arquivos.
dest-change = Alterar
dest-continue = Revisar o plano

## M23 Approve the move you want

plan-eyebrow = Sua mudança
plan-title = Aprove a mudança que você quer.
plan-body = O PCTwin faz só o que este plano diz.
plan-what-title = O que você escolheu
# $size is already formatted, for example "120 GB". $count is passed but not shown.
plan-what-body = { $size } de arquivos, das pastas que você escolheu, para os lugares que você definiu.
plan-where-body =
    { NUMBER($count) ->
        [one] Para { $count } lugar no notebook novo.
        [many] Para { $count } de lugares no notebook novo.
       *[other] Para { $count } lugares no notebook novo.
    }
plan-hand-title = Antes de começar
plan-hand-body =
    { NUMBER($count) ->
        [one] { $count } etapa. Ela aparece no seu relatório no final.
        [many] { $count } de etapas. Elas aparecem no seu relatório no final.
       *[other] { $count } etapas. Elas aparecem no seu relatório no final.
    }
plan-hand-none = Nada, até onde o PCTwin consegue saber agora.
plan-old-safe = Seu notebook antigo continua como está. Os arquivos são copiados, nunca tirados dele.
plan-edit = Alterar o plano
plan-edit-note = Se você alterar o plano, vai aprová-lo de novo antes que algo novo comece.
plan-approve = Aprovo este plano, incluindo o que cabe a mim fazer.
plan-not-approved = Você ainda não aprovou este plano, então nada será movido.
plan-changed = Você alterou o plano, então aprove-o de novo antes que algo novo seja movido.
plan-start = Começar a mudança

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = Voltar
# $n is a count of files or folders.
plan-number = { NUMBER($n) }
# $n is a number already rounded for reading, for example 18.4.
plan-size-b = { NUMBER($n) } bytes
plan-size-kb = { NUMBER($n) } KB
plan-size-mb = { NUMBER($n) } MB
plan-size-gb = { NUMBER($n) } GB
plan-size-tb = { NUMBER($n) } TB
# $size is a size already written out, for example "18.4 GB". Used when part of a folder could not be read.
plan-size-at-least = pelo menos { $size }
plan-size-unknown = tamanho desconhecido
plan-role-home = Pasta pessoal
plan-role-desktop = Área de Trabalho
plan-role-documents = Documentos
plan-role-downloads = Downloads
plan-role-pictures = Imagens
plan-role-music = Músicas
plan-role-videos = Vídeos
plan-role-public = Pasta pública
plan-role-app-data = Dados de apps
plan-role-other = Outra pasta
# $done and $total are counts of places this search has looked at and has to look at.
scan-places = { $done } de { $total } lugares procurados
scan-card-title = Procurando nos lugares que você escolheu
scan-pausing = Pausando depois da pasta atual. Tudo o que foi encontrado até agora está salvo.
overview-row-attention = Precisa de atenção
files-status-work = Trabalho ou escola
files-work-body = Sua empresa ou escola gerencia esta pasta, então ela fica de fora, a menos que você a marque.
# $count is how many files in the folder are only online.
files-online-only =
    { NUMBER($count) ->
        [one] { $count } arquivo está só online, então não é copiado.
        [many] { $count } de arquivos estão só online, então não são copiados.
       *[other] { $count } arquivos estão só online, então não são copiados.
    }
files-no-match = Nenhuma pasta tem esse nome.
dest-status-none = Escolha um lugar
dest-status-readonly = Não aceita gravação
dest-status-format = Arquivo grande demais
# $name is the folder that needs a place.
dest-none = Escolha um lugar para { $name } no notebook novo.
# $place is a drive on the new laptop that cannot be written to.
dest-readonly = Não é possível gravar em { $place }, então nada pode ser copiado para lá. Escolha outro lugar.
# $place is the drive. Some drive formats cannot hold a single very large file.
dest-format = Um arquivo nestas pastas é grande demais para o jeito como { $place } está preparado. Escolha outro lugar.
# $folder is a folder on the new laptop and $drive the drive it is on.
plan-place = { $folder } em { $drive }
plan-group-to = Vai para { $place }
plan-version = Versão do plano { $revision }
plan-details = Mais detalhes
plan-preview-title = Antes de começar
plan-preview-fits = Há espaço livre suficiente neste notebook.
plan-preview-short = Este notebook precisa de mais { $size } de espaço livre. Libere espaço antes de começar, ou a transferência para quando o disco encher.
plan-preview-space-unknown = O PCTwin não conseguiu ler quanto espaço livre este notebook tem.
plan-preview-time-one =
    { NUMBER($minutes) ->
        [one] Cerca de { $minutes } minuto.
        [many] Cerca de { $minutes } de minutos.
       *[other] Cerca de { $minutes } minutos.
    }
plan-preview-time = Cerca de { $low } a { $high } minutos.
plan-preview-renamed =
    { NUMBER($count) ->
        [one] { $count } arquivo terá um nome um pouco diferente, porque este notebook não aceita o nome dele. O relatório indica isso.
        [many] { $count } de arquivos terão nomes um pouco diferentes, porque este notebook não aceita os nomes deles. O relatório indica cada um.
       *[other] { $count } arquivos terão nomes um pouco diferentes, porque este notebook não aceita os nomes deles. O relatório indica cada um.
    }
plan-preview-too-big =
    { NUMBER($count) ->
        [one] { $count } arquivo é grande demais para o disco deste notebook e não será copiado.
        [many] { $count } de arquivos são grandes demais para o disco deste notebook e não serão copiados.
       *[other] { $count } arquivos são grandes demais para o disco deste notebook e não serão copiados.
    }
plan-preview-refused =
    { NUMBER($count) ->
        [one] { $count } arquivo tem um nome ou pasta que este notebook não aceita, e não será copiado.
        [many] { $count } de arquivos têm um nome ou pasta que este notebook não aceita, e não serão copiados.
       *[other] { $count } arquivos têm um nome ou pasta que este notebook não aceita, e não serão copiados.
    }
plan-preview-runs-at-sign-in =
    { NUMBER($count) ->
        [one] { $count } arquivo abriria sozinho quando alguém entrar neste notebook. Confira se você espera isso.
        [many] { $count } de arquivos abririam sozinhos quando alguém entrar neste notebook. Confira se você espera isso.
       *[other] { $count } arquivos abririam sozinhos quando alguém entrar neste notebook. Confira se você espera isso.
    }
# $reference is a short code that names exactly this version of the plan.
plan-reference = Referência { $reference }
# $count is how many files are affected in each line below.
plan-hand-cloud-only =
    { NUMBER($count) ->
        [one] { $count } arquivo está só online. O PCTwin não baixa esse arquivo. Deixe-o disponível no notebook antigo antes ou deixe-o de fora.
        [many] { $count } de arquivos estão só online. O PCTwin não baixa esses arquivos. Deixe-os disponíveis no notebook antigo antes ou deixe-os de fora.
       *[other] { $count } arquivos estão só online. O PCTwin não baixa esses arquivos. Deixe-os disponíveis no notebook antigo antes ou deixe-os de fora.
    }
plan-hand-unreadable =
    { NUMBER($count) ->
        [one] { $count } item não pôde ser aberto, então não vai ser movido. Permita o acesso ou deixe-o de fora.
        [many] { $count } de itens não puderam ser abertos, então não vão ser movidos. Permita o acesso ou deixe-os de fora.
       *[other] { $count } itens não puderam ser abertos, então não vão ser movidos. Permita o acesso ou deixe-os de fora.
    }
plan-hand-work-managed =
    { NUMBER($count) ->
        [one] { $count } item é gerenciado pela sua empresa ou escola e fica de fora. Marque a pasta dele para incluí-lo.
        [many] { $count } de itens são gerenciados pela sua empresa ou escola e ficam de fora. Marque as pastas deles para incluí-los.
       *[other] { $count } itens são gerenciados pela sua empresa ou escola e ficam de fora. Marque as pastas deles para incluí-los.
    }
plan-blocked-someone-unmapped = Alguém no notebook antigo ainda não tem uma conta escolhida no novo. Escolha para onde vão os arquivos de cada pessoa e depois volte.
plan-blocked-account-used-twice = Duas pessoas acabariam na mesma conta no notebook novo. Dê a cada pessoa a sua própria conta.
plan-blocked-invalid = Este plano não pode ser aprovado como está. Volte e confira o que você escolheu.
plan-blocked-fix = Escolher contas

## Tray menu

tray-show = Mostrar o PCTwin
tray-quit = Sair do PCTwin

## F01 secondary actions and their coming-soon pages

welcome-restore = Restaurar configuração completa
welcome-rescue = Resgatar um notebook
welcome-quick-send = Envio rápido

soon-eyebrow = Em breve
soon-restore-title = Restaurar configuração completa
soon-restore-body = Monte um notebook de novo a partir de uma configuração completa que você salvou depois de uma mudança. Isso chega em uma versão futura.
soon-rescue-title = Resgatar um notebook
soon-rescue-body = Recupere arquivos de um notebook que não liga, conectando o disco dele a um que funciona. Isso chega em uma versão futura.

## X01 Move progress, compact

progress-eyebrow = Sua mudança
progress-title = Preparando tudo para você.
progress-title-paused = Mudança pausada
progress-paired = Pareados
progress-percent =
    .aria-label = Progresso da mudança
# $done and $total are already formatted sizes, for example "81.6 GB" and "120 GB".
progress-amount = { $done } de { $total }
# $range is a measured range, already formatted, for example "15 to 20 minutes".
progress-time-left = Faltam cerca de { $range }
progress-time-measuring = Calculando o tempo restante…
# $speed is measured and already formatted, for example "62 MB/s".
progress-speed = { $speed }, medido
progress-count-needs-action =
    { NUMBER($count) ->
        [one] { $count } item precisa da sua ação
        [many] { $count } de itens precisam da sua ação
       *[other] { $count } itens precisam da sua ação
    }
progress-current-copying = Copiando agora: { $name }
progress-current-checking = Verificando agora: { $name }
progress-needs-action =
    { NUMBER($count) ->
        [one] { $count } item precisa da sua ação. O resto continua sendo movido.
        [many] { $count } de itens precisam da sua ação. O resto continua sendo movido.
       *[other] { $count } itens precisam da sua ação. O resto continua sendo movido.
    }
progress-see-needs-action = Ver o que precisa de você
progress-interrupted = A conexão com o outro notebook caiu. O PCTwin está tentando reconectar. Nada do que foi copiado e verificado se perde.
progress-low-battery = A bateria deste notebook está fraca. Ligue-o na tomada para a transferência não parar.
progress-battery-paused = A bateria deste notebook está quase acabando, então a transferência foi pausada com segurança. Ligue-o na tomada e depois continue. Nada se perde.
progress-old-safe = Seu notebook antigo continua como está.
progress-pause = Pausar mudança
progress-compact = Manter compacto
progress-expand = Ver a mudança toda
progress-stop = Parar a mudança

## X02 Move progress, expanded

progress-all-title = Veja a mudança toda.
progress-all-body = Cada item mostra de onde vem e como está indo.
progress-metric-overall = progresso geral
progress-metric-committed = copiados e verificados
progress-metric-estimate = estimativa
progress-col-item = Item
progress-col-from = Origem
progress-col-status = Situação
progress-state-waiting = Esperando
progress-state-copying = Copiando
progress-state-verified = Verificado
progress-state-checking = Verificando
progress-state-paused = Pausado
progress-checked-note = Um item só conta como copiado depois de ser verificado.

## F09 Mini panel

mini-title = Sua mudança, por perto.
mini-title-paused = Mudança pausada.
mini-what = movendo seus arquivos selecionados
mini-progress =
    .aria-label = Progresso da mudança
mini-pause = Pausar
mini-resume = Retomar
mini-open = Abrir mudança
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = Seus arquivos estão a caminho.
mini-what-send = enviando os arquivos selecionados
mini-open-send = Abrir envio
mini-progress-send =
    .aria-label = Progresso do envio

## X05 Paused at a safe point

pause-title = Pausado em um ponto seguro.
pause-title-finishing = Pausando em um ponto seguro…
pause-body = Tudo o que foi copiado e verificado é mantido. O resto espera.
pause-finishing = Terminando { $name } primeiro, para não ficar copiado pela metade.
pause-kept-title = Arquivos
pause-kept-body = Tudo o que foi copiado e verificado até agora é mantido.
pause-waiting-title = Esperando
pause-waiting-body =
    { NUMBER($count) ->
        [one] Ainda falta copiar { $count } item. Nada se perde.
        [many] Ainda falta copiar { $count } de itens. Nada se perde.
       *[other] Ainda falta copiar { $count } itens. Nada se perde.
    }
pause-status-kept = Pausado
pause-status-waiting = Esperando
pause-note = Pausar não desfaz nada. Seu notebook antigo continua como está.
pause-resume = Retomar a mudança salva

## X10 Stop the remaining move? (danger dialog)

cancel-title = Parar o resto da mudança?
cancel-body = Parar mantém tudo o que foi copiado e verificado. Isso não desfaz a mudança.
cancel-copied-title = Arquivos copiados
cancel-copied-body = Ficam no notebook novo.
cancel-rest-title = O resto da mudança
cancel-rest-body = Nada novo começa. Os arquivos ainda não copiados ficam só no notebook antigo.
cancel-partial-title = Um arquivo copiado em parte
cancel-partial-body = Nunca é mantido como se estivesse completo.
cancel-old-title = Notebook antigo
cancel-old-body = Continua como está.
cancel-ack = Entendo que parar não desfaz o que foi copiado
cancel-stop = Parar e ver o relatório
cancel-keep = Continuar a mudança

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = Resultado da mudança
outcome-title = Boas-vindas ao seu próximo notebook.
outcome-body = Veja o que aconteceu com tudo o que você aprovou.
outcome-files-title = Arquivos selecionados
outcome-status-partial = Feito em parte
outcome-next-title = O que fazer agora
outcome-old-safe = O que não terminou continua listado aqui. Nada foi removido do seu notebook antigo.
outcome-report = Ver o relatório completo
outcome-home = Voltar ao início
move-fallback-folder = Do seu notebook antigo
move-stop = Parar a transferência
move-get-help = Obter ajuda
pickup-eyebrow = Sua transferência
pickup-title = Retome com segurança.
pickup-body = Uma transferência neste notebook parou no meio, talvez porque ele reiniciou ou ficou sem energia. Organize primeiro e depois pareie os notebooks de novo para continuar.
pickup-unfinished-title = Cópias não terminadas
pickup-unfinished-body =
    { NUMBER($count) ->
        [one] { $count } arquivo estava sendo copiado quando a transferência parou.
        [many] { $count } de arquivos estavam sendo copiados quando a transferência parou.
       *[other] { $count } arquivos estavam sendo copiados quando a transferência parou.
    }
pickup-unfinished-badge = A organizar
pickup-tidy = Organizar
pickup-tidying = Organizando. As cópias terminadas são verificadas e colocadas no lugar; nada seu é alterado.
pickup-done = Tudo organizado. Pareie os notebooks de novo para continuar: o que já está aqui é reconhecido e não é copiado duas vezes.
pickup-failed = A organização não funcionou desta vez. Nada seu foi alterado. Tente de novo, ou feche o PCTwin neste notebook e abra-o de novo antes.
pickup-put-in-place = colocados no lugar
pickup-kept = mantidos para continuar
pickup-not-finished =
    { NUMBER($count) ->
        [one] { $count } arquivo não pôde ser terminado. A próxima transferência o copia de novo.
        [many] { $count } de arquivos não puderam ser terminados. A próxima transferência os copia de novo.
       *[other] { $count } arquivos não puderam ser terminados. A próxima transferência os copia de novo.
    }
pickup-left-for-later =
    { NUMBER($count) ->
        [one] { $count } arquivo está em um disco que não está conectado agora. Conecte-o e organize de novo.
        [many] { $count } de arquivos estão em um disco que não está conectado agora. Conecte-o e organize de novo.
       *[other] { $count } arquivos estão em um disco que não está conectado agora. Conecte-o e organize de novo.
    }
pickup-later = Agora não
move-status-kept = Mantido
move-wait-eyebrow = Sua transferência
move-wait-scanning-title = Procurando seus arquivos.
move-wait-scanning-body = Seu notebook antigo está olhando suas pastas. Ele só as lê: nada é aberto, alterado ou removido.
move-wait-approval-title = Aguardando o notebook novo.
move-wait-approval-body = Seus arquivos estão prontos. Confira o plano no notebook novo e aprove-o para começar.
move-wait-list-title = Aguardando o notebook antigo.
move-wait-list-body = Seu notebook antigo está procurando seus arquivos. Quando terminar, o plano aparece aqui para você aprovar. Isso pode levar alguns minutos.
move-wait-starting-title = Iniciando a transferência.
move-wait-starting-body = O plano foi aprovado. A cópia começa em instantes.
move-wait-admin-title = Aguardando um administrador.
move-wait-admin-body = Este notebook está pedindo a aprovação de um administrador para mover também as coisas das outras pessoas. Responda na janela que o notebook abriu.
move-others-not-approved = O administrador não aprovou, então desta vez só as suas coisas serão movidas. As das outras pessoas ficam onde estão.
move-others-failed = O PCTwin não conseguiu iniciar o que precisa para mover as coisas das outras pessoas, então desta vez só as suas serão movidas. Nada delas foi alterado.
move-others-not-here = Mover as coisas de outras pessoas a partir deste notebook ainda não está pronto nesta versão, então só as suas serão movidas.
who-here-title = Quem vai mudar?
who-here-body = Suas coisas sempre são movidas. Marque qualquer outra pessoa deste notebook cujas coisas também devam ir. O PCTwin nunca mostra os arquivos delas para você.
who-here-you = { $name } (você)
who-here-you-hint = Sempre vai
who-here-approval = Mover as coisas de outra pessoa precisa da aprovação de um administrador, uma só vez, para todas as pessoas que você marcar. O notebook vai pedir na própria janela.
who-here-not-here = Mover as coisas de outras pessoas a partir deste notebook ainda não está pronto nesta versão. Só as suas serão movidas.
move-ended-eyebrow = Sua transferência
move-ended-kept-title = Já copiado
move-ended-kept-body = Tudo o que foi copiado e verificado antes disso fica no notebook novo.
move-ended-old-title = Seu notebook antigo
move-ended-old-body = Nada nele foi alterado ou removido.
move-ended-old-badge = Sem alterações
move-ended-next = Para tentar de novo, pareie os notebooks outra vez. Os arquivos já copiados e verificados são reconhecidos e não são copiados duas vezes.
move-lost-title = A conexão com seu outro notebook foi perdida.
move-lost-body = Os notebooks pararam de se comunicar, então a transferência não pôde continuar. Confira se os dois estão ligados, ativos e perto um do outro.
move-not-approved-title = A transferência não começou.
move-not-approved-old = O plano não foi aprovado no notebook novo, então nada foi copiado.
move-not-approved-new = Você não aprovou o plano, então nada foi copiado.
move-failed-title = Algo deu errado.
move-failed-body = A transferência não pôde continuar. Se acontecer de novo, Mais detalhes, abaixo, diz o que deu errado.
move-refused-title = A transferência não pôde começar.
move-refused-not-paired = Seus notebooks não estão mais pareados. Pareie-os de novo para começar a transferência.
move-refused-busy = Já há uma transferência em andamento neste notebook.
move-refused-not-moving = Não há nenhuma transferência em andamento neste notebook.
move-refused-plan-changed = O plano mudou antes de ser aprovado. Pareie os notebooks de novo para ver o plano novo.
move-refused-failed = Não foi possível preparar as pastas deste notebook para a transferência. Pareie os notebooks de novo para tentar outra vez.
drive-problem-title = A transferência precisa da sua atenção.
drive-problem-body = As cópias salvas em { $place } não bateram com os originais quando o PCTwin as verificou, várias vezes. O disco pode estar falhando, cheio ou desconectado, então a cópia lá está pausada.
drive-problem-kept-title = Já copiado e verificado
drive-problem-waiting-title = Ainda falta copiar para { $place }
drive-problem-badge = Pausado
drive-problem-fix = Confira se o disco está conectado e tem espaço livre, depois continue. Nada do que você escolheu fica de fora para contornar isso. Você também pode parar a transferência.
drive-problem-carry-on = Continuar

## R02 Your move, item by item (detailed report)

report-title = Sua mudança, item por item.
report-body = O que aconteceu com cada item, por quê, de onde veio e para onde foi.
report-filter-label = Mostrar
report-col-item = Item
report-col-result = Resultado
report-col-details = Verificação ou o que fazer
report-from = De: { $path }
report-to = Para: { $path }
report-none = Nenhum item tem este resultado.
report-more-details = Mais detalhes

# Evidence: how a copied item was checked.
evidence-read-back = Verificado lendo a cópia de volta do disco.
evidence-memory = Verificado pela memória deste notebook, não pelo disco.
evidence-already-there = Já estava no notebook novo: o mesmo arquivo já estava lá, então não foi copiado de novo.
evidence-recovered = Verificado antes de o notebook reiniciar, e verificado de novo depois.
evidence-new-laptop = Verificado pelo notebook novo.
evidence-memory-explain = A cópia foi lida de volta e era igual, mas pela memória deste notebook, então o que foi lido pode ter vindo da memória e não do disco. Um defeito no próprio disco pode não aparecer aqui, então abrir o arquivo uma vez é uma boa verificação.
evidence-not-checked = Não verificado, porque não terminou.
# Machine-translated 16 October 2026.
why-memory-cannot-skip-cache = Este disco não deixou o PCTwin lê-lo diretamente, então a verificação passou pela memória deste notebook. Alguns discos de rede e pendrives USB funcionam assim.
why-memory-not-shown = Este disco não conseguiu mostrar que a leitura do PCTwin chegou mesmo até ele, então a verificação conta como feita pela memória deste notebook.
why-memory-no-way = O sistema deste notebook não tem como ler este disco diretamente, então a verificação passou pela memória dele.
why-memory-stored-specially = Este arquivo está guardado de um jeito especial, por exemplo compactado ou criptografado, então o sistema o lê pela memória.
why-memory-too-small = Este arquivo é muito pequeno, então o disco o guarda junto com os próprios registros, e o sistema o lê pela memória.

# Reasons: why an item did not finish. Each says what happened, then what to do.
reason-not-enough-space = O notebook novo ficou sem espaço. Libere espaço nele e comece uma nova mudança que inclua este item.
reason-changed-during-move = Este arquivo continuou mudando no notebook antigo enquanto era copiado. Feche qualquer app que esteja usando o arquivo e comece uma nova mudança que o inclua.
reason-unreadable = O notebook antigo não deixou o PCTwin ler este item. Verifique se ele abre no notebook antigo e comece uma nova mudança que o inclua.
reason-not-verified = A cópia não era igual ao original, então não foi mantida. Comece uma nova mudança que inclua o item. Se falhar de novo, o original pode estar danificado.
reason-refused-by-safety-check = A verificação de segurança do notebook novo recusou este item por causa do nome, do lugar ou do tamanho. O original continua no notebook antigo, então você pode copiá-lo de outro jeito se precisar.
reason-name-not-allowed = Este nome não pode ser usado no notebook novo, e não foi possível criar um nome novo seguro. Mude o nome no notebook antigo e comece uma nova mudança que inclua o item.
reason-install-failed = O app não foi instalado. Instale-o pelo site do desenvolvedor.
reason-no-equivalent = Não existe versão deste app para o sistema do notebook novo. Procure um app parecido que funcione lá.
reason-sign-in = Já está no lugar, mas você precisa entrar na conta. Abra e entre na conta.
reason-activation = O app já está no lugar, mas precisa da licença no notebook novo. Abra o app e ative-o com sua licença ou conta.
reason-permission = O notebook novo precisa dar permissão primeiro. Abra o item e escolha Permitir quando o pedido aparecer.
reason-online-only = Este arquivo está guardado só online, então não estava no notebook antigo para ser copiado. Baixe-o no notebook novo pelo app que o guarda online.
reason-manual-install = Este app precisa vir do site do desenvolvedor. Baixe e instale por lá.
reason-restart = O notebook novo precisa reiniciar para terminar. Reinicie quando quiser.
reason-finish-by-hand = O PCTwin não consegue fazer esta etapa por você. Siga os passos mostrados para ela neste relatório.

## Item results (every item ends with one of these)

status-copied = Copiado
status-reinstalled = Reinstalado
status-converted = Convertido
status-needs-action = Precisa de ação
status-skipped-by-user = Pulado por você
status-failed = Falhou

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
        [one] { $count } item não pôde ser copiado
        [many] { $count } de itens não puderam ser copiados
       *[other] { $count } itens não puderam ser copiados
    }
progress-queue-empty = Nada começou ainda.
progress-approved-job = Mudança aprovada
progress-route-old = Notebook antigo
progress-route-new = Notebook novo
progress-panel-pausing = Pausando
outcome-metric-size = copiados e verificados
outcome-metric-action = precisam da sua ação
progress-percent-unknown = O PCTwin ainda está calculando o tamanho desta mudança.
mini-title-send-paused = Envio pausado.
cancel-ack-hint = Marque a caixa acima para ativar o botão de parar.
progress-state-placing = Colocando no lugar
progress-state-existing = Já estava lá
outcome-stopped-title = Sua mudança foi interrompida.
outcome-stopped-body = Você parou a mudança. O que foi copiado e verificado antes disso é mantido. O resto não foi copiado.
# $done and $total are counts of items: what was copied and checked, and everything the person approved.
outcome-files-summary = { $done } de { $total } itens copiados e verificados.
outcome-files-memory =
    { NUMBER($count) ->
        [one] { $count } verificado pela memória deste notebook, não pelo disco.
        [many] { $count } de itens verificados pela memória deste notebook, não pelo disco.
       *[other] { $count } verificados pela memória deste notebook, não pelo disco.
    }
report-show-more = Mostrar mais
# $shown and $total are counts of items.
report-showing = Mostrando { $shown } de { $total }

## Component kit

dialog-close = Fechar
toast-dismiss = Dispensar
code-label = Código de seis dígitos

## Shared Quick Send words

qs-eyebrow = Envio rápido
qs-eyebrow-receiving = Recebendo neste notebook
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = De { $from } para { $to }
qs-route-sender = Enviando · este notebook
qs-route-receiver = Recebendo · mesmo Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } item selecionado · { $size }
        [many] { $count } de itens selecionados · { $size }
       *[other] { $count } itens selecionados · { $size }
    }
qs-tally-badge = Envio por perto

## Q01 Quick Send home

qs-home-title = Alguns arquivos. Direto para o outro.
qs-home-body = Envie alguns arquivos para um notebook por perto.
qs-send-title = Enviar arquivos
qs-send-body = Escolha arquivos ou pastas neste notebook.
qs-receive-title = Receber arquivos
qs-receive-body = Deixe um notebook por perto enviar arquivos para este.
qs-history-title = Seus recibos
qs-history-body = Veja para onde foram os arquivos recebidos e o que não terminou.
qs-home-action = Escolher arquivos
qs-receiving = Pronto para receber. Outros notebooks por perto veem este como { $label }.
qs-receive-stop = Parar de receber
qs-no-nearby = Nenhum notebook encontrado por perto. Abra o PCTwin no outro notebook e mantenha os dois no mesmo Wi-Fi.

## Q02 Choose files and folders

qs-choose-title = O que você vai enviar?
qs-choose-body = Escolha só os arquivos e pastas que quer compartilhar.
qs-drop =
    .aria-label = Área para soltar arquivos e pastas
qs-drop-title = Solte arquivos ou pastas aqui
qs-drop-body = Ou adicione com Adicionar arquivos ou Adicionar pastas.
qs-drop-active = Solte para adicionar.
qs-add-files = Adicionar arquivos
qs-add-folders = Adicionar pastas
qs-remove =
    .aria-label = Remover { $name }
# $kind is the file type, for example "PDF document" or "8 photos".
qs-file-detail = { $size } · { $kind }
qs-empty = Nada escolhido ainda. Adicione arquivos ou pastas para enviar.
qs-online-only = Alguns arquivos estão só online. Deixe-os disponíveis neste notebook primeiro e depois envie.
qs-protected = O PCTwin não consegue ler { $name }. Permita o acesso ou tire o item da lista.
qs-allow = Permitir acesso
qs-choose-continue = Verificar estes arquivos

## Q03 Check what you are sharing

qs-check-title = Confira o que você vai compartilhar.
qs-check-body = Revise estes arquivos antes que saiam deste notebook.
qs-check-included = Incluído
qs-check-shared-note = O outro notebook vê os nomes, tamanhos e datas destes arquivos.
qs-check-changed = { $name } mudou desde que você escolheu. Verifique de novo.
qs-check-ack = Já conferi estes arquivos
qs-check-ack-hint = Tire tudo o que for privado antes de enviar.
qs-check-continue = Escolher o outro notebook

## Q04 Who should receive it?

qs-who-title = Quem vai receber?
qs-who-body = Escolha o nome mostrado no outro notebook.
qs-who-nearby = Por perto · mesmo Wi-Fi.
qs-who-code = Usar código de pareamento
qs-who-code-body = Digite o código mostrado no notebook que envia.
qs-who-same-label = Dois notebooks por perto têm este nome. Confira o nome na tela do outro notebook.
qs-who-busy = { $label } está ocupado com outro envio. Espere um pouco e tente de novo.
qs-who-note = O outro notebook aceita os arquivos e escolhe para onde eles vão.
qs-who-action = Ver meu código de envio

## Q08 Incoming offer (receiving laptop)

qs-offer-title = Alguém quer enviar arquivos para você.
qs-offer-body = Confira o que está sendo enviado antes de aceitar.
qs-offer-note = O PCTwin salva só os arquivos que você aceitar. Ele nunca os abre nem os inicia.
qs-offer-accept = Aceitar e escolher uma pasta
qs-offer-decline = Recusar estes arquivos
qs-offer-declined = Você recusou. Nada foi salvo neste notebook.
qs-offer-changed = Quem envia mudou os arquivos. Confira a nova lista antes de aceitar.
qs-offer-gone = { $label } não está mais disponível. Nada foi salvo neste notebook.

## Q09 Where should the files land?

qs-land-title = Onde os arquivos devem ficar?
qs-land-body = Escolha a pasta neste notebook.
qs-land-folder-label = Pasta de recebimento
qs-land-choose = Escolher outra pasta
qs-land-space-title = Espaço disponível
qs-land-space = { $free } disponíveis · { $needed } necessários.
qs-land-status-fits = Cabe
qs-land-status-no-space = Espaço insuficiente
qs-land-no-space = Esta pasta tem { $free } livres, mas estes arquivos precisam de { $needed }. Escolha outra pasta ou libere espaço.
qs-land-unavailable = Esta pasta não está disponível. Conecte o disco dela ou escolha outra pasta.
qs-land-denied = O PCTwin não tem permissão para salvar nesta pasta. Escolha outra pasta.
qs-land-note = Se já houver um arquivo com o mesmo nome, você escolhe o que acontece antes de qualquer coisa ser salva.
qs-land-confirm = Usar esta pasta para estes arquivos
qs-land-confirm-hint = Só os arquivos que você aceitou são salvos aqui.
qs-land-use = Usar esta pasta

## Q10 Keep your existing work safe

qs-conflict-title = Mantenha seu trabalho atual seguro.
qs-conflict-body = Escolha o que acontece quando já existe um arquivo com o mesmo nome.
qs-conflict-col-file = Item
qs-conflict-col-existing = Já está lá
qs-conflict-col-choice = Opção
# $when is already formatted, for example "yesterday".
qs-conflict-existing = Editado { $when }
qs-conflict-none = Nenhuma cópia existente
qs-conflict-select =
    .aria-label = O que acontece com { $name }
qs-keep-both = Manter os dois
qs-skip = Pular este item
qs-add-new = Adicionar novo
qs-keep-both-note = Manter os dois deixa o arquivo que já está lá como está. O novo é salvo como { $newName }.
qs-no-conflicts = Nenhum destes arquivos está na pasta ainda. Nada será substituído.
qs-conflict-target-changed = Se o arquivo que já está lá mudar ou não puder ser aberto, ele espera até você verificar de novo.
qs-conflict-approve = Aprovo estas opções
qs-conflict-continue = Confirmar estas opções

## Q11 Ready to send these files?

qs-approve-title = Tudo pronto para enviar estes arquivos?
qs-approve-body = Confira exatamente o que está sendo enviado antes de começar.
qs-approve-receiver-title = Outro notebook
qs-approve-receiver-waiting = Esperando o outro notebook aceitar e escolher uma pasta.
qs-approve-receiver-ready = O outro notebook aceitou e escolheu uma pasta e o que fazer com os arquivos que já estão lá.
qs-approve-files-title = Arquivos
qs-approve-files-same = Não mude estes arquivos até que sejam enviados.
qs-approve-files-changed = Alguns arquivos mudaram desde que você os conferiu. Confira de novo.
qs-approve-existing-title = Arquivos existentes
qs-approve-existing-body = Manter os dois, como escolhido.
qs-status-waiting = Esperando
qs-status-accepted = Aprovado
qs-status-unchanged = Revisar
qs-status-changed = Alterado
qs-approve-ack = Enviar estes arquivos
qs-approve-ack-hint = Aprovo estes arquivos e este notebook.
qs-approve-start = Começar a enviar
qs-approve-cleared = Você mudou os arquivos, então { $label } precisa aceitá-los de novo.
qs-approve-withdrawn = { $label } não aceita mais este envio. Nada foi enviado.

## Q12 Send progress

qs-sending-title = Seus arquivos estão a caminho.
qs-sending-body = As cópias vão direto de um notebook para o outro.
qs-sending-progress =
    .aria-label = Enviando os arquivos selecionados
qs-sending-current = Enviando agora: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, medido
qs-step-copying = Copiando
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } item selecionado recebido.
        [many] { $done } de itens selecionados recebidos.
       *[other] { $done } itens selecionados recebidos.
    }
qs-step-checking = Verificando
qs-step-checking-body = Cada arquivo é comparado com o original.
qs-step-receipt = Recibo
qs-step-receipt-body = No final, veja para onde foram os arquivos e o que não terminou.
qs-sending-paused = Pausado. Os arquivos já recebidos e verificados são mantidos.
qs-sending-disconnected = A conexão com { $label } caiu. Os arquivos já verificados são mantidos. Tentando de novo automaticamente…
qs-sending-needs-action =
    { NUMBER($count) ->
        [one] { $count } item precisa da sua ação.
        [many] { $count } de itens precisam da sua ação.
       *[other] { $count } itens precisam da sua ação.
    }
qs-pause = Pausar envio
qs-resume = Retomar
qs-compact = Manter compacto
qs-stop = Parar este envio
qs-sending-complete-title = O envio terminou
qs-sending-complete-body = Cada arquivo que chegou é igual ao original. Veja onde eles foram salvos.
qs-sending-receipt = Ver o recibo

## Q14 Handover receipt (both laptops)

qs-receipt-title = Aqui está seu recibo.
qs-receipt-body = Veja exatamente o que chegou e onde ficou.
qs-receipt-box-title = Seus arquivos chegaram
qs-receipt-box-body = Cada cópia foi comparada com o original e é igual.
qs-receipt-arrived =
    { NUMBER($count) ->
        [one] { $count } item selecionado recebido
        [many] { $count } de itens selecionados recebidos
       *[other] { $count } itens selecionados recebidos
    }
qs-receipt-size = { $size } iguais ao original
qs-receipt-saved-as = { $path }
qs-receipt-not-arrived = Nem tudo desta pasta chegou e foi verificado.
qs-receipt-status-verified = Verificado
qs-receipt-status-unfinished = Não terminado
qs-receipt-not-completed = Não concluído.
qs-receipt-stopped-row = Recebido e verificado antes de parar.
qs-receipt-partial =
    { NUMBER($count) ->
        [one] { $count } item ainda precisa de atenção.
        [many] { $count } de itens ainda precisam de atenção.
       *[other] { $count } itens ainda precisam de atenção.
    }
qs-receipt-stopped = Este envio foi interrompido. Os arquivos que chegaram inteiros ficam na pasta que você escolheu.
qs-receipt-not-opened = O PCTwin não abriu nem iniciou nenhum arquivo recebido.
qs-receipt-ack = Confirmar recibo
# $when is already formatted, for example "today at 14:05".
qs-receipt-acked = Recibo confirmado { $when }.
qs-receipt-ack-note = Confirmar este recibo não significa que os arquivos foram abertos ou usados.
qs-receipt-offline = { $label } está offline, então ainda não viu este recibo. O recibo está salvo neste notebook.
qs-receipt-missing = Enviar o que falta
qs-receipt-missing-body = Verifique só os arquivos que não chegaram.
qs-receipt-history = Todos os recibos
qs-receipt-done = Voltar ao Envio rápido

## Q19 Finish the missing part

qs-missing-title = Termine a parte que falta.
qs-missing-body = Use o recibo anterior para ver o que não chegou.
qs-missing-status-there = Chegou
qs-missing-status-missing = Precisa de ação
qs-missing-there-body = Chegou e é igual ao original.
qs-missing-missing-body = Verifique os arquivos que faltam ou que não terminaram.
qs-missing-note = Os arquivos que chegaram são mantidos. Arquivos novos ou alterados precisam da sua aprovação de novo.
qs-missing-ack = Enviar os arquivos que faltam
qs-missing-ack-hint = O PCTwin verifica os arquivos de novo, e o outro notebook os aceita de novo.
qs-missing-complete = Tudo já chegou. Não há mais nada para enviar.
qs-missing-unavailable = O recibo anterior não foi encontrado, então o PCTwin não consegue saber o que falta. Comece um novo envio.
qs-missing-continue = Verificar o que será enviado

## Q20 Stop this send? (and decline)

qs-stop-title = Parar este envio?
qs-stop-body = Confira o que já chegou antes de parar.
qs-stop-arrived-title = Arquivos que chegaram inteiros
qs-stop-arrived-body = Ficam na pasta que você escolheu.
qs-stop-rest-title = Ainda não enviados
qs-stop-rest-body = Nada novo é enviado depois que você parar.
qs-stop-partial-title = Arquivos que chegaram em parte
qs-stop-partial-body = Nunca são salvos com o nome real, então nunca são confundidos com cópias completas.
qs-stop-status-keep = Manter
qs-stop-status-stop = Parar
qs-stop-status-review = Revisar
qs-stop-ack = Parar este envio
qs-stop-ack-hint = Os arquivos que chegaram inteiros não são apagados.
qs-stop-confirm = Parar e ver o recibo
qs-stop-keep = Continuar enviando
qs-decline-title = Recusar estes arquivos?
qs-decline-body = Nada é salvo neste notebook, e quem envia fica sabendo que você recusou.
qs-decline-confirm = Recusar
qs-already-finished = Este envio já terminou. Veja o recibo para saber o que chegou.

## F08 Your moves, kept on this laptop (history)

history-title = Suas mudanças, guardadas neste notebook.
history-body = Abra um relatório de novo ou continue uma mudança salva.
history-row-move = { $from } → { $to }
history-row-sent = Enviado para { $label }
history-row-received = Recebido de { $label }
history-status-finished = Concluído
history-status-steps-left =
    { NUMBER($count) ->
        [one] Concluído, { $count } etapa para você
        [many] Concluído, { $count } de etapas para você
       *[other] Concluído, { $count } etapas para você
    }
history-status-paused = Pausado
history-status-stopped = Interrompido com segurança
history-status-partial = Nem tudo chegou
history-open-report = Relatório
history-open-receipt = Recibo
history-continue = Continuar
history-export = Exportar histórico
history-exported = Histórico salvo como { $path }.
history-export-failed = Não foi possível salvar o histórico aí. Escolha outra pasta e tente de novo.
history-note = Este histórico fica neste notebook. Nada é enviado para lugar nenhum, a menos que você o exporte e compartilhe por conta própria.
history-empty = Sua primeira mudança e o relatório dela vão aparecer aqui.
history-empty-title = Nenhuma mudança registrada ainda
history-start = Começar uma nova mudança

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = O Envio rápido ainda não está ativado nesta versão. Nada foi enviado ou recebido.
qs-tally-counting =
    { NUMBER($count) ->
        [one] { $count } item selecionado · tamanho ainda sendo calculado
        [many] { $count } de itens selecionados · tamanho ainda sendo calculado
       *[other] { $count } itens selecionados · tamanho ainda sendo calculado
    }
qs-gone = { $name } não está mais lá. Pode ter sido movido ou apagado. Desmarque o item ou coloque-o de volta e adicione de novo.
qs-choose-why-problems = Alguns itens marcados ainda não podem ser enviados. Desmarque-os ou resolva como indicado para continuar.
qs-check-missing = { $name } não está mais lá. Volte e remova o item ou coloque-o de volta.
qs-check-unreadable = O PCTwin não consegue mais ler { $name }. Permita o acesso ou volte e remova o item.
qs-check-show-all =
    { NUMBER($count) ->
        [one] Mostrar o arquivo
        [many] Mostrar todos os { $count } de arquivos
       *[other] Mostrar todos os { $count } arquivos
    }
qs-check-change = Mudar o que vou enviar
qs-land-none = Nenhuma pasta escolhida ainda.
qs-land-space-unknown = O PCTwin não conseguiu medir o espaço livre aqui. Estes arquivos precisam de { $needed }.
qs-conflict-existing-unknown = Já está lá
qs-conflict-choice-open = Ainda não escolhido
qs-conflict-why = Escolha o que acontece com cada arquivo e marque a caixa para continuar.
qs-approve-folder-title = Pasta de recebimento
qs-status-recipient-choice = Escolha de quem recebe
qs-sending-card = Enviando os arquivos selecionados
qs-progress-of = de { $total }
qs-receipt-partial-title = Nem tudo chegou
qs-receipt-metric-items =
    { NUMBER($count) ->
        [one] item selecionado recebido
        [many] de itens selecionados recebidos
       *[other] itens selecionados recebidos
    }
qs-receipt-metric-size = verificados com a origem
qs-receipt-sent-as = Enviado como { $path }
qs-receipt-renamed = Já havia um arquivo com esse nome, então este recebeu um nome novo.
qs-receipt-chip-already = Já estava lá
qs-receipt-already = Um arquivo idêntico já estava lá ({ $path }), então não foi enviado de novo.
qs-receipt-chip-declined = Não aceito
qs-receipt-declined-send = O outro notebook não aceitou este arquivo, então ele não foi enviado.
qs-receipt-declined-receive = Você não aceitou este arquivo, então ele não foi salvo.
qs-receipt-more = Mais detalhes
qs-receipt-empty = Nada foi enviado, então não há nada para mostrar.
qs-receipt-already-count =
    { NUMBER($count) ->
        [one] { $count } item já estava lá como arquivo idêntico
        [many] { $count } de itens já estavam lá como arquivos idênticos
       *[other] { $count } itens já estavam lá como arquivos idênticos
    }
qs-missing-new-send = Começar um novo envio
history-open-name = { $what }: { $title }

## R05 Desfazer: o que desfazer fará em cada disco, e como cada cópia terminou

undo-here-removes = Desfazer apaga as cópias feitas pelo PCTwin que você não alterou. Tudo o que você alterou é mantido.
undo-here-to-trash = Desfazer move para o Lixo as cópias feitas pelo PCTwin que você não alterou. Tudo o que você alterou é mantido, e nada é apagado.
undo-here-keeps-fat = Desfazer vai manter todas as cópias em { $drive }. Este disco usa FAT ou exFAT, que reutiliza os números que diferenciam os arquivos, então o PCTwin não tem como saber ao certo quais arquivos são seus. Você mesmo pode apagá-los.
undo-here-keeps-other = Desfazer vai manter todas as cópias em { $drive }. Neste tipo de disco, o PCTwin não consegue mover nem apagar arquivos com segurança. Você mesmo pode apagá-los.
undo-here-more = Mais detalhes
undo-here-more-trash = Um Mac não deixa o PCTwin ter certeza de que nenhum outro app ainda está gravando em um arquivo, então o PCTwin nunca apaga nenhum. Ele move cada cópia para o Lixo do mesmo disco. Você pode recuperá-la no Lixo.
undo-here-more-fat = Em discos FAT e exFAT, um arquivo novo pode receber o número que um arquivo apagado tinha. O PCTwin só age sobre arquivos que consegue provar que são seus e que não mudaram, então mantém estes.
undo-done-trashed = Movido para o Lixo como “{ $name }”. Você pode recuperá-lo no Lixo.
undo-kept-fat = Mantido. Este disco (FAT ou exFAT) reutiliza os números que diferenciam os arquivos, então o PCTwin não tem como saber ao certo se esta é a cópia dele. Se não precisar dela, você mesmo pode apagá-la.
undo-kept-no-trash = Mantido. O PCTwin não conseguiu colocar esta cópia no Lixo do disco dela. Se não precisar dela, você mesmo pode apagá-la.
