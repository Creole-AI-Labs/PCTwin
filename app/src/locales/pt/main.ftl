# PCTwin screens, Brazilian Portuguese (pt). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = Página inicial do PCTwin
language-label = Idioma
motion-label = Movimento
motion-system = Movimento: como o sistema
motion-reduced = Movimento: reduzido
motion-off = Movimento: desligado

# The opening (Wordplay); its tagline is the home headline.
opening-label = O PCTwin está abrindo
opening-status = Abrindo o PCTwin.
opening-skip = Pular a intro
opening-tagline = Envie arquivos ou mude tudo de uma vez.

footer-free = Grátis. Não precisa de conta.
footer-help = Ajuda

## F01 Home

welcome-title-line1 = Envie arquivos
welcome-title-line2 = ou mude tudo de uma vez.
welcome-body = Envie alguns arquivos para um notebook por perto, ou leve seus arquivos, apps e configurações para o seu notebook novo.
welcome-start = Começar uma mudança completa

## F01 Home: the cards, My history, a move to resume, files waiting

home-quick-send-body = Enviar ou receber arquivos
home-move-body = Leve sua configuração
home-restore-body = Traga de volta
home-rescue-body = Salve o que importa
home-history = Meu histórico
home-history-body = Suas mudanças e seus relatórios
name-title = Dê um nome a este notebook.
name-body = Os outros notebooks por perto vão encontrá-lo por este nome. Você pode mudá-lo depois, ao parear.
name-use = Usar este nome
home-update-title = Uma nova versão está disponível.
home-update-action = Atualizar agora
home-updating = Atualizando. O PCTwin vai fechar e abrir de novo sozinho.
home-update-failed = Não foi possível instalar a atualização. O PCTwin funciona como antes; ela será oferecida de novo da próxima vez.
home-update-busy = Termine sua mudança primeiro. Depois atualize o PCTwin.
home-resume-body = Seu progresso está salvo.
home-resume-action = Retomar
    .aria-label = Retomar sua mudança
home-waiting =
    { NUMBER($count) ->
        [one] { $count } arquivo esperando por { $to }
        [many] { $count } de arquivos esperando por { $to }
       *[other] { $count } arquivos esperando por { $to }
    }

## Header: this laptop, the language and motion control, Help

header-this-laptop = Seu notebook

## F06 Old or new laptop

role-back = Voltar
role-eyebrow = Começar uma mudança completa
role-title = Qual notebook é este?
role-body = Abra o PCTwin nos dois notebooks. Depois diga qual é este.
role-old-title = Este é o meu notebook antigo
role-old-body = Enviar meus arquivos, apps e configurações.
role-new-title = Este é o meu notebook novo
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
check-speed-unknown = O PCTwin não conseguiu medir a conexão desta vez. Os arquivos são movidos mesmo assim; só falta a estimativa de tempo.
check-space-unknown = O PCTwin não conseguiu ler o espaço livre no notebook novo.
check-power-unknown = O PCTwin não conseguiu saber se { $label } está ligado na tomada.
check-internet-unknown = O PCTwin não conseguiu saber se este notebook está online. Tudo bem: copiar arquivos não precisa de internet.
check-again = Verificar de novo
check-estimate-note = A estimativa de tempo não é uma promessa fixa.
check-continue = Escolher o que mover
# Added 23 October 2026: the short checklist both laptops show before the move.
check-before-title = Antes de a mudança começar
check-before-lids-title = Deixe as duas tampas abertas
check-before-lids-body = Uma tampa fechada pode colocar o notebook em repouso, e isso pausa a mudança.
check-before-near-title = Fique perto do roteador Wi-Fi
check-before-near-body = Deixe os dois notebooks perto dele enquanto suas coisas são movidas.

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
reconnect-nothing-new = Nada novo chegou há algum tempo. O PCTwin continua tentando sozinho, ou você pode parar a transferência. Tudo o que já foi copiado e verificado fica guardado.
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
pair-old-label-title = O nome deste notebook
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
pair-old-same-role = Outro notebook por perto também está configurado como o notebook antigo. Um dos dois precisa ser o notebook novo.
pair-old-switch = Este é o notebook novo
pair-new-none-same-role = O outro notebook também escolheu Notebook novo? Então escolha Notebook antigo nele, ou troque este.
pair-new-switch = Este é o notebook antigo
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
help-home-eyebrow = Ajuda
help-home-title = Ajuda para o que você está fazendo.
help-home-body = Guias curtos para o que mais costuma atrapalhar. Funcionam sem internet.
help-home-more = Mais ajuda em getpctwin.com
help-guide-find-title = Os notebooks não se encontram
help-guide-find-body = Conecte os dois notebooks à mesma Wi-Fi, não a uma rede de convidados. Desligue qualquer VPN enquanto pareia. Se ainda não funcionar, ative o roteador do celular e conecte os dois notebooks a ele.
help-guide-mac-network-title = Seu Mac pergunta sobre a rede local
help-guide-mac-network-body = Escolha Permitir para o PCTwin encontrar seu outro notebook. Se escolheu Não Permitir, abra Ajustes do Sistema, depois Privacidade e Segurança, depois Rede Local, e ative o PCTwin.
help-guide-firewall-title = O Windows pergunta sobre o firewall
help-guide-firewall-body = Permita o PCTwin em redes privadas, como a Wi-Fi de casa. O PCTwin só fala com seu outro notebook, na sua própria rede.
help-guide-stopped-title = Uma transferência parou no meio
help-guide-stopped-body = Nada do que foi copiado e verificado se perde, e o notebook antigo não mudou. Pareie os notebooks de novo: os arquivos que já estão no novo são reconhecidos e não são copiados duas vezes.
help-guide-others-title = Mover as coisas de outras pessoas
help-guide-others-body = Marque as pessoas em Quem vai mudar? no notebook antigo. Um administrador aprova uma vez em cada notebook. Você nunca vê os arquivos delas. A senha temporária de uma conta nova aparece uma vez, em uma janela do próprio notebook; a pessoa escolhe a dela no primeiro acesso.
help-guide-code-title = Pedir ajuda a alguém
help-guide-code-body = Em qualquer página de problema, escolha Obter ajuda. O PCTwin cria um código curto que diz o que deu errado, sem nomes, arquivos ou senhas, para você mandar por WhatsApp ou qualquer app.

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
plan-group-move = Mover { $folder }
plan-place-change = Escolher outro lugar
plan-place-usual = Usar o lugar habitual
plan-place-free = { $size } livres lá
plan-place-no-room = Não há espaço livre suficiente lá. Escolha outro lugar ou desmarque esta pasta.
plan-place-no-safe-name = O PCTwin não consegue salvar arquivos com segurança nessa unidade. Escolha outro lugar.
plan-place-cannot-open = O PCTwin não conseguiu abrir essa pasta. Escolha outro lugar.
plan-place-locked = Essa unidade está bloqueada. Desbloqueie-a primeiro e escolha-a de novo.
plan-version = Versão do plano { $revision }
plan-details = Mais detalhes
plan-preview-title = Antes de começar
plan-preview-fits = Há espaço livre suficiente neste notebook.
plan-preview-fits-about = Deve haver espaço livre suficiente neste notebook. Isso conta cerca de { $extra } para os apps que o PCTwin instala e os dados deles.
# Added 23 October 2026: what moves first, from the move's own order. $places is a list of folder names.
plan-preview-first = Vamos mover isto primeiro, para que seu notebook novo fique pronto mais cedo: { $places }.
plan-preview-short = Este notebook precisa de mais { $size } de espaço livre. Libere espaço antes de começar, ou a transferência para quando o disco encher.
plan-preview-short-about = Este notebook precisa de cerca de { $size } a mais de espaço livre, contando cerca de { $extra } para os apps que o PCTwin instala e os dados deles. Libere espaço antes de começar, ou a transferência para quando o disco encher.
plan-preview-space-unknown = O PCTwin não conseguiu ler quanto espaço livre este notebook tem.
plan-preview-synced = Neste notebook, { $folder } fica guardada no { $service }. Os arquivos transferidos para lá serão enviados ao { $service } e usarão seu espaço lá.
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
plan-hand-too-many-files =
    { NUMBER($count) ->
        [one] { $count } pasta tem mais arquivos do que cabem em uma só mudança, por isso não vai ser movida. Depois desta mudança, envie-a sozinha com o Envio rápido ou copie-a com um pen drive USB.
        [many] { $count } de pastas têm mais arquivos do que cabem em uma só mudança, por isso não vão ser movidas. Depois desta mudança, envie cada uma sozinha com o Envio rápido ou copie-as com um pen drive USB.
       *[other] { $count } pastas têm mais arquivos do que cabem em uma só mudança, por isso não vão ser movidas. Depois desta mudança, envie cada uma sozinha com o Envio rápido ou copie-as com um pen drive USB.
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
soon-move-title = Mudança completa
soon-move-body = Leve tudo do seu notebook antigo para um novo: seus arquivos, aplicativos e configurações. Isso chega em uma versão futura. Por enquanto, use o Envio rápido para enviar arquivos.

## X01 Move progress, compact

progress-eyebrow = Sua mudança
progress-title = Preparando tudo para você.
progress-title-old = Enviando suas coisas para o novo notebook.
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
# Added 23 October 2026: T14, one tip under the progress bar, about every 12 seconds.
tips-label = Dica
tips-pause = Pausar dicas
tips-resume = Mostrar mais dicas
tips-next = Próxima dica
tip-battery = Este notebook está usando a bateria. Ligue-o na tomada se puder, para que uma mudança longa não seja interrompida.
tip-closer = A conexão está lenta. Aproximar os notebooks do roteador Wi-Fi pode deixá-la mais rápida.
tip-lids = Deixe a tampa deste notebook aberta. Uma tampa fechada pode colocá-lo em repouso, e isso pausa a mudança.
tip-keep-using = Você pode continuar usando este notebook. O PCTwin só lê os arquivos dele.
tip-pause = Você pode pausar a mudança a qualquer momento e continuar depois.
tip-power-cut = Se a energia acabar, ligue os dois notebooks e abra o PCTwin. Ele continua de onde parou, e nada do que foi copiado e verificado se perde.

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
# Added 23 October 2026: the after-move check on R01, counted the way the scan counts.
outcome-kinds-title = Verificado depois da mudança
outcome-kind-photos = Fotos
outcome-kind-videos = Vídeos
outcome-kind-music = Músicas
outcome-kind-documents = Documentos
outcome-kind-count = { $copied } de { $planned } copiados e verificados
outcome-next-title = O que fazer agora
outcome-old-safe = O que não terminou continua listado aqui. Nada foi removido do seu notebook antigo.
outcome-report = Ver o relatório completo
outcome-home = Voltar ao início
outcome-person-copied =
    { NUMBER($count) ->
        [one] { $count } arquivo copiado.
        [many] { $count } de arquivos copiados.
       *[other] { $count } arquivos copiados.
    }
outcome-person-not-done =
    { NUMBER($count) ->
        [one] { $count } não terminou.
        [many] { $count } de arquivos não terminaram.
       *[other] { $count } não terminaram.
    }
outcome-person-at-sign-in = Estarão no lugar quando { $name } entrar neste notebook da próxima vez.
move-fallback-folder = Do seu notebook antigo
move-stop = Parar a transferência
move-before-open = Enquanto espera: antes de deixar este notebook
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
ready-eyebrow = Boas-vindas
ready-title = Suas coisas estão prontas.
ready-body = Suas coisas do notebook antigo foram movidas para este e agora estão nas suas próprias pastas.
ready-landed = arquivos nas suas pastas
ready-where-title = Onde estão
ready-where-body = Em Documentos, Imagens, Músicas, Vídeos e suas outras pastas de sempre, como no notebook antigo.
ready-kept =
    { NUMBER($count) ->
        [one] { $count } arquivo foi guardado à parte, porque já havia um com esse nome. Os dois foram mantidos.
        [many] { $count } de arquivos foram guardados à parte, porque já havia outros com esses nomes. Todos foram mantidos.
       *[other] { $count } arquivos foram guardados à parte, porque já havia outros com esses nomes. Todos foram mantidos.
    }
ready-failed =
    { NUMBER($count) ->
        [one] Não foi possível colocar { $count } arquivo nas suas pastas. Peça a quem fez a transferência para movê-lo de novo.
        [many] Não foi possível colocar { $count } de arquivos nas suas pastas. Peça a quem fez a transferência para movê-los de novo.
       *[other] Não foi possível colocar { $count } arquivos nas suas pastas. Peça a quem fez a transferência para movê-los de novo.
    }
ready-done = Concluído
move-status-kept = Mantido
move-wait-eyebrow = Sua transferência
move-wait-scanning-title = Procurando seus arquivos.
move-wait-scanning-body = Seu notebook antigo está olhando suas pastas. Ele só as lê: nada é aberto, alterado ou removido.
move-found = Encontrado até agora: { $size } das suas coisas.
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
move-others-allow-in-settings = Primeiro, um administrador precisa permitir o PCTwin na página dos Ajustes do Sistema que acabou de abrir. Desta vez só as suas coisas serão movidas; quando o PCTwin estiver permitido, a sua próxima mudança poderá levar também as coisas das outras pessoas.
approve-prompt = O PCTwin vai ler os arquivos de { $names } para levá-los ao novo notebook. Um administrador deste notebook aprova só esta mudança.
who-here-title = Quem vai mudar?
who-here-body = Suas coisas sempre são movidas. Marque qualquer outra pessoa deste notebook cujas coisas também devam ir. O PCTwin nunca mostra os arquivos delas para você.
who-here-you = { $name } (você)
who-here-you-hint = Sempre vai
who-here-approval = Mover as coisas de outra pessoa precisa da aprovação de um administrador, uma só vez, para todas as pessoas que você marcar. O notebook vai pedir na própria janela.
who-here-not-here = Mover as coisas de outras pessoas a partir deste notebook ainda não está pronto nesta versão. Só as suas serão movidas.
place-title = Para onde vão as coisas deles?
place-body = Também vieram coisas de outras pessoas do notebook antigo. Escolha para cada pessoa uma conta deste notebook, uma conta nova ou deixe-a de fora. Você nunca vê os arquivos delas.
place-size =
    { NUMBER($count) ->
        [one] { $count } arquivo, { $size }
        [many] { $count } de arquivos, { $size }
       *[other] { $count } arquivos, { $size }
    }
place-select = Para onde vão as coisas de { $name }
place-note = As contas novas, e guardar as coisas delas até entrarem, precisam da aprovação de um administrador uma só vez, depois que você aprovar o plano. A senha temporária de cada conta nova aparece uma vez, em uma janela do próprio notebook.
place-todo =
    { NUMBER($count) ->
        [one] Escolha para onde vão as coisas de mais { $count } pessoa.
        [many] Escolha para onde vão as coisas de mais { $count } de pessoas.
       *[other] Escolha para onde vão as coisas de mais { $count } pessoas.
    }
fda-title = Seu Mac está mantendo algumas pastas privadas.
fda-body =
    { NUMBER($count) ->
        [one] Não foi possível olhar { $count } dos seus lugares por causa das configurações de privacidade do Mac. Deixe o PCTwin entrar e verifique de novo, para nada seu ficar de fora.
        [many] Não foi possível olhar { $count } dos seus lugares por causa das configurações de privacidade do Mac. Deixe o PCTwin entrar e verifique de novo, para nada seu ficar de fora.
       *[other] Não foi possível olhar { $count } dos seus lugares por causa das configurações de privacidade do Mac. Deixe o PCTwin entrar e verifique de novo, para nada seu ficar de fora.
    }
fda-step-1 = Abra Ajustes do Sistema e depois Privacidade e Segurança.
fda-step-2 = Escolha Acesso Total ao Disco.
fda-step-3 = Ative o PCTwin. O Mac pode pedir sua senha.
fda-open = Abrir estes ajustes
fda-note = O PCTwin só lê suas pastas para copiá-las. Ele nunca altera nada neste notebook.
fda-again = Verificar de novo
fda-without = Continuar sem elas
password-title = PCTwin: uma conta nova
password-body =
    { $name } agora tem uma conta neste notebook.
    
    Nome de acesso: { $account }
    Senha temporária: { $password }
    
    Passe isso para { $name }. No primeiro acesso, a pessoa escolhe a própria senha. Anote agora: o PCTwin não a guarda.
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
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = Outro sistema
move-update-here-title = Atualize o PCTwin neste notebook
move-update-here-body = Este notebook tem um PCTwin mais antigo que o outro, então eles não podem mover arquivos juntos. Atualize o PCTwin aqui e comece de novo. Nada foi copiado.
move-update-there-title = Atualize o PCTwin no outro notebook
move-update-there-body = O outro notebook tem um PCTwin mais antigo que este, então eles não podem mover arquivos juntos. Atualize o PCTwin lá e comece de novo. Nada foi copiado.
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
# Added 23 October 2026: what the report says beside a line's result (changed since the scan).
report-note-changed-since-scan = Mudou desde a verificação: foi copiado como está agora.
report-note-no-longer-there = Não está mais lá: foi movido, renomeado ou apagado no notebook antigo antes de poder ser enviado.
report-note-time-not-kept = Data não mantida: esta unidade não consegue guardar a data da última alteração, por isso mostra a data em que foi copiado.

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
reason-drive-locked = A unidade de destino está bloqueada. Desbloqueie-a (por exemplo, com a senha ou a chave de recuperação do BitLocker) e transfira de novo.
reason-sign-in = Já está no lugar, mas você precisa entrar na conta. Abra e entre na conta.
reason-activation = O app já está no lugar, mas precisa da licença no notebook novo. Abra o app e ative-o com sua licença ou conta.
reason-permission = O notebook novo precisa dar permissão primeiro. Abra o item e escolha Permitir quando o pedido aparecer.
reason-online-only = Este arquivo está guardado só online, então não estava no notebook antigo para ser copiado. Baixe-o no notebook novo pelo app que o guarda online.
reason-manual-install = Este app precisa vir do site do desenvolvedor. Baixe e instale por lá.
reason-restart = O notebook novo precisa reiniciar para terminar. Reinicie quando quiser.
reason-finish-by-hand = O PCTwin não consegue fazer esta etapa por você. Siga os passos mostrados para ela neste relatório.
reason-parts-missing = Alguns arquivos desta pasta não chegaram, por isso ela não está completa no notebook novo. Eles aparecem neste relatório com o motivo. Envie-os de novo ou copie a pasta de outro jeito.
reason-moved-file-missing = Esta configuração aponta para um arquivo ou uma pasta que não está no notebook novo com o nome que a transferência deu a ele, então o PCTwin deixou a configuração como estava. Abra o app e escolha o arquivo de novo.

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
outcome-broke-title = Sua mudança terminou antes da hora.
outcome-broke-body = Algo deu errado entre os notebooks, então a mudança não pôde continuar. O que foi copiado e verificado antes disso é mantido. O resto não foi copiado.
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
qs-eyebrow-receiving = Recebendo no seu notebook
qs-label-this = Este notebook
qs-label-other = O outro notebook
qs-pick-failed = Não foi possível abrir o seletor. Tente de novo.
qs-pick-too-many = São mais arquivos do que um único envio comporta. Desmarque algumas pastas ou envie-as em partes.
qs-problem-lost-title = A conexão entre os notebooks caiu.
qs-problem-failed-title = Desta vez não foi possível enviar os arquivos.
qs-problem-body = O que já foi recebido e verificado fica onde está. Pareie os notebooks de novo para tentar outra vez.
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = De { $from } para { $to }
qs-route-sender = Enviando
qs-route-receiver = Recebendo · mesmo Wi-Fi
qs-tally =
    { NUMBER($count) ->
        [one] { $count } item selecionado · { $size }
        [many] { $count } de itens selecionados · { $size }
       *[other] { $count } itens selecionados · { $size }
    }
qs-tally-badge = Envio por perto

## Q01 Quick Send home

qs-home-title = Envie arquivos para um notebook por perto.
qs-home-body = Ou receba arquivos de um.
qs-send-title = Enviar arquivos
qs-send-body = Escolha arquivos ou pastas no seu notebook.
qs-receive-title = Receber arquivos
qs-receive-body = Deixe um notebook por perto enviar arquivos para o seu.
qs-home-action = Escolher arquivos
qs-receiving = Pronto para receber. Notebooks por perto veem o seu como { $label }.
qs-receive-stop = Parar de receber
qs-no-nearby = Nenhum notebook encontrado por perto. Abra o PCTwin no notebook que envia e deixe os dois no mesmo Wi-Fi.

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
qs-online-only = Alguns arquivos estão só online. Deixe-os disponíveis no seu notebook primeiro e depois envie.
qs-protected = O PCTwin não consegue ler { $name }. Permita o acesso ou tire o item da lista.
qs-allow = Permitir acesso
qs-choose-continue = Continuar

## Q12 Send progress

qs-sending-title = Seus arquivos estão a caminho.
qs-sending-body = As cópias vão direto para o notebook que recebe.
qs-sending-progress =
    .aria-label = Enviando arquivos
qs-sending-current = Enviando agora: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }, medido
qs-step-copying = Copiando
qs-step-copying-body =
    { NUMBER($done) ->
        [one] { $done } arquivo enviado.
        [many] { $done } de arquivos enviados.
       *[other] { $done } arquivos enviados.
    }
qs-step-checking = Verificando
qs-step-checking-body = Cada arquivo é comparado com o original.
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
qs-stop-keep = Continuar enviando

## F08 Your moves, kept on this laptop (history)

history-title = Suas mudanças, guardadas neste notebook.
history-body = Abra um relatório de novo ou continue uma mudança salva.
history-row-move = { $from } → { $to }
history-status-finished = Concluído
history-status-steps-left =
    { NUMBER($count) ->
        [one] Concluído, { $count } etapa para você
        [many] Concluído, { $count } de etapas para você
       *[other] Concluído, { $count } etapas para você
    }
history-status-paused = Pausado
history-status-stopped = Interrompido com segurança
history-open-report = Relatório
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
qs-sending-card = Enviando arquivos
qs-progress-of = de { $total }
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

## Quick Send (26 October 2026): pairing in Quick Send's words, where received files go, and the Done page

qs-pair-old-eyebrow = Enviando
qs-pair-old-body = No notebook que recebe, escolha Receber arquivos e depois digite este código.
qs-pair-old-waiting = Esperando o notebook que recebe…
qs-pair-old-label-hint = O notebook que recebe vai encontrar o seu por este nome.
qs-pair-old-steps-title = No notebook que recebe
qs-pair-old-step-1 = Abra o PCTwin, escolha Envio rápido e depois Receber arquivos.
qs-pair-old-left = O notebook que recebe parou o pareamento antes de um número ser escolhido, então nada foi compartilhado. Aqui está um novo código.
qs-pair-network-hint = Escolha a rede em que o notebook que recebe está.
qs-pair-choose-body = Olhe o notebook que recebe. Escolha o número que ele mostra.
qs-pair-choose-renewed = Novos números apareceram. Escolha o que está no notebook que recebe.
qs-pair-choose-note = Se o notebook que recebe não mostrar nenhum número, ou se nenhum destes for igual, escolha Nenhum é igual.
qs-pair-heads-up-firewall = O Windows pode perguntar se o PCTwin pode aceitar conexões. Escolha Permitir para que o notebook que recebe consiga chegar ao seu.
qs-pair-new-eyebrow = Recebendo
qs-pair-new-searching-title = Procurando o notebook que envia…
qs-pair-new-searching-body = Deixe o PCTwin aberto no notebook que envia, no mesmo Wi-Fi que o seu.
qs-pair-new-search-failed-title = Não foi possível procurar o notebook que envia.
qs-pair-new-list-title = Qual notebook está enviando?
qs-pair-new-list-body = Escolha o nome que aparece na tela do notebook que envia.
qs-pair-new-same-label = Outro notebook por perto tem este nome. Confira a tela do notebook que envia.
qs-pair-new-none-title = Nenhum notebook enviando foi encontrado.
qs-pair-new-none-body = Verifique se o PCTwin está aberto no notebook que envia mostrando um código e se os dois notebooks estão no mesmo Wi-Fi. Depois procure de novo.
qs-pair-new-code-hint = Leia na tela de { $label }. Ele muda a cada minuto.
qs-pair-new-number-title = Escolha este número em { $label }.
qs-pair-new-number-body = Três números aparecem em { $label }. Escolha este lá.
qs-pair-new-number-waiting = Esperando { $label }…
qs-pairing-code-invalid = O código precisa ter seis dígitos. Confira a tela de { $label } e digite de novo.
qs-pairing-code-wrong = Esse código não confere. Confira a tela de { $label } e tente de novo.
qs-pairing-code-expired = Esse código mudou. Digite o novo que está na tela de { $label }.
qs-pairing-busy = Não dá para parear com { $label } agora: ele está pareando com outro notebook. Espere um pouco e tente de novo.
qs-pairing-wait = Um novo código está sendo preparado em { $label }. Espere e depois digite o novo código.
qs-pairing-locked = O pareamento parou depois de tentativas erradas demais. Em { $label }, escolha Começar de novo.
qs-pairing-unreachable = Seu notebook não conseguiu alcançar { $label }. Verifique se os dois estão no mesmo Wi-Fi e tente de novo.
qs-pairing-wrong-number = Esse número não é igual ao do notebook que recebe. Nada foi compartilhado. Digite o novo código no notebook que recebe para tentar de novo.
qs-pairing-no-match = Os números não combinaram em { $label }. Nada foi compartilhado. Digite o novo código que está na tela dele.
qs-pairing-timed-out = Nenhum número foi escolhido a tempo. Tente de novo com o novo código de { $label }.
qs-pairing-connection-lost-new = A conexão com { $label } caiu. Nada foi compartilhado.
qs-pairing-choose-network = Seu notebook está em mais de uma rede. Escolha aquela em que o notebook que recebe está:
qs-pairing-failed-new = O pareamento não funcionou desta vez. Nada foi compartilhado. Digite o novo código da tela de { $label }.
qs-folder-line = Os arquivos serão salvos em { $folder }.
qs-folder-change = Mudar
qs-folder-title = Onde salvar os arquivos?
qs-folder-none = O PCTwin não pode usar a pasta dos arquivos recebidos. Escolha uma pasta para os arquivos e depois conecte.
qs-folder-choose = Escolher uma pasta
qs-folder-failed = O PCTwin não pode salvar arquivos nessa pasta. Escolha outra.
qs-done-sent-title = Seus arquivos foram enviados.
qs-done-here-title = Seus arquivos chegaram.
qs-done-partial-title = Alguns arquivos não chegaram.
qs-done-stopped-title = Este envio foi parado.
qs-done-empty-title = Nada foi enviado.
qs-done-arrived =
    { NUMBER($count) ->
        [one] { $count } arquivo · { $size } chegou igual ao original
        [many] { $count } de arquivos · { $size } chegaram iguais aos originais
       *[other] { $count } arquivos · { $size } chegaram iguais aos originais
    }
qs-done-arrived-count =
    { NUMBER($count) ->
        [one] { $count } arquivo chegou igual ao original
        [many] { $count } de arquivos chegaram iguais aos originais
       *[other] { $count } arquivos chegaram iguais aos originais
    }
qs-done-already =
    { NUMBER($count) ->
        [one] { $count } arquivo já estava lá, idêntico, então não foi enviado de novo.
        [many] { $count } de arquivos já estavam lá, idênticos, então não foram enviados de novo.
       *[other] { $count } arquivos já estavam lá, idênticos, então não foram enviados de novo.
    }
qs-done-stopped-body = Os arquivos que chegaram antes da parada foram verificados e ficam onde foram salvos. Nada mais foi enviado.
qs-done-folder = Salvos em { $folder }
qs-done-open-folder = Abrir pasta
qs-done-dates-new =
    { NUMBER($count) ->
        [one] Este arquivo mostra a hora em que chegou, por isso está no topo da pasta.
        [many] Estes { $count } de arquivos mostram a hora em que chegaram, por isso estão no topo da pasta.
       *[other] Estes { $count } arquivos mostram a hora em que chegaram, por isso estão no topo da pasta.
    }
qs-done-keep-dates = Manter as datas originais
qs-done-dates-kept =
    { NUMBER($count) ->
        [one] { $count } arquivo voltou a ter a data original.
        [many] { $count } de arquivos voltaram a ter a data original.
       *[other] { $count } arquivos voltaram a ter a data original.
    }
qs-done-dates-failed = Não foi possível devolver as datas originais. Os arquivos estão bem e continuam no lugar.
qs-done-open-failed = Não foi possível abrir a pasta. Ela pode ter sido movida ou apagada.
qs-done-not-opened = O PCTwin não abriu nem iniciou nenhum arquivo recebido.
qs-done-problems-title = Arquivos que precisam de atenção
qs-done-not-arrived = Não chegou.
qs-done-not-completed = Não foi enviado antes da parada.
qs-done-needs-action = Está aqui, mas não terminado. O que fazer está em Mais detalhes.
qs-done-mark-not-kept = Está aqui, mas esta unidade não conseguiu marcá-lo como baixado de outro notebook. Abra com cuidado.
qs-done-see-files = Ver os arquivos
qs-done-renamed = Salvo como { $name }, porque já havia um arquivo com esse nome.
qs-done-identical = Já havia um arquivo idêntico, então ele não foi enviado de novo.
qs-done-send-missing = Enviar os arquivos que faltam
qs-done-send-missing-body = Só os arquivos que não chegaram. Primeiro você pareia os notebooks de novo.
qs-done-done = Concluído
qs-stop-now = Parar o envio
qs-stop-finished = Este envio já terminou.
qs-route-receiving = Recebendo
qs-receiving-title = Os arquivos estão chegando.
qs-receiving-body = As cópias vêm direto de { $label }.
qs-receiving-card = Recebendo arquivos
qs-receiving-progress =
    .aria-label = Recebendo arquivos
qs-receiving-current = Recebendo: { $name }
qs-step-receiving-body =
    { NUMBER($done) ->
        [one] { $done } arquivo chegou.
        [many] { $done } de arquivos chegaram.
       *[other] { $done } arquivos chegaram.
    }
qs-tally-files =
    { NUMBER($count) ->
        [one] { $count } arquivo · { $size }
        [many] { $count } de arquivos · { $size }
       *[other] { $count } arquivos · { $size }
    }
qs-receive-pause = Pausar
qs-label-yours = Seu notebook
qs-label-receiving = Notebook que recebe
qs-label-sending = Notebook que envia
qs-peer-receiving = o notebook que recebe
qs-peer-sending = o notebook que envia
qs-pair-old-label-title = Nome do seu notebook
qs-pair-old-rename-label = Nome para o seu notebook
qs-pair-old-step-2 = Se aparecer uma lista, escolha o nome do seu notebook.
qs-pair-old-locked-body = Depois de cinco códigos errados, o pareamento parou para proteger o seu notebook. Nada foi compartilhado. Se foi você, escolha Começar de novo para ter um novo código.
qs-pair-new-needs-update = Ele tem uma versão antiga do PCTwin. Atualize o PCTwin nele e procure de novo.
qs-pairing-no-network = Seu notebook não está em nenhuma rede Wi-Fi ou com fio. Conecte-o e tente de novo.
qs-problem-room-title = Não há espaço suficiente em { $folder }.
qs-problem-room-body = Estes arquivos precisam de { $needed }, e lá podem ser usados { $room }. Nada foi salvo. Libere espaço ou escolha outra pasta e tente de novo.
qs-done-refused-title = O notebook que recebe não aceitou estes arquivos.
qs-done-refused-body = Talvez não tenha espaço suficiente. Nada foi enviado.

## Telas da fase 2 (lista de tarefas 2.8): partes comuns

guard-state-not-started = Ainda não começou
guard-state-preparing = Preparando
guard-state-awaiting-person = Esperando a sua escolha
guard-state-awaiting-system = Esperando a aprovação do seu sistema
guard-state-awaiting-maker = Esperando o criador do app ou do serviço online
guard-state-working = Trabalhando
guard-state-checking = Verificando o que foi feito
guard-state-applying = Colocando as coisas no lugar
guard-state-pause-requested = Vai pausar no próximo ponto seguro. Ainda não está pausado.
guard-state-paused = Pausado em um ponto seguro
guard-state-source-unavailable = O notebook antigo ou o disco não está conectado
guard-state-blocked = Algo precisa ser resolvido primeiro
guard-state-partial = Feito em parte
guard-state-retrying = Tentando de novo
guard-state-reconciling = Verificando até onde chegou
guard-state-failed = Isto não funcionou
guard-state-complete = Pronto
guard-state-cancelled = Interrompido. O que já foi feito foi mantido.
guard-refused-unavailable = O PCTwin não conseguiu verificar isto agora. Nada foi feito. Tente de novo daqui a pouco.
guard-refused-wrong-job = Isto era de outra mudança. Nada foi feito.
guard-refused-old-session = Esta tela estava desatualizada, então nada foi feito. Aqui está o que é verdade agora.
guard-refused-plan-changed = O plano mudou enquanto você olhava, então nada foi feito. Olhe de novo.
guard-refused-not-now = Isso não pode ser feito nesta etapa. Nada foi feito.
guard-refused-not-shown = Essa opção não está mais nesta tela. Nada foi feito.
guard-progress-label = Progresso
guard-progress-mark = { $percent }% concluído
guard-progress-of = { $done } de { $total }
guard-progress-unknown-total = { $done } até agora. O total ainda não é conhecido.
guard-progress-none = Nada foi medido ainda.
guard-and-more =
    { $count ->
        [one] E mais { $count }, listado no relatório.
        [many] E mais { $count }, listados no relatório.
       *[other] E mais { $count }, listados no relatório.
    }
guard-not-known-yet = Ainda não se sabe
guard-free = { $free } livres
guard-free-unknown = Espaço livre desconhecido
guard-items-size = { $items } itens, { $size }
guard-try-again = Tentar de novo
guard-pause = Pausar
guard-resume = Continuar
guard-check-again = Verificar de novo
guard-strip-plan = Seu plano
guard-strip-report = Seu relatório
guard-strip-permission = Aprovação
guard-strip-rescue = Resgate
guard-strip-undo = Desfazer
guard-strip-checking = Verificando

## X09 verificação depois de reiniciar, e estado indisponível

guard-checking-title = Verificando até onde chegou
guard-checking-body = O PCTwin está olhando o que foi concluído antes de fechar. Suas escolhas voltam assim que ele souber.
guard-unavailable-title = Estado indisponível
guard-unavailable-body = O PCTwin não consegue dizer agora o que esta mudança está fazendo. Nada aparece como feito até que ele consiga. Verifique de novo daqui a pouco.

## 2.9 Antes de mudarmos qualquer coisa

guard-changes-title = Antes de mudarmos qualquer coisa
guard-changes-body = Isto é tudo o que a mudança vai alterar neste notebook, e o que o desfazer pode fazer com cada coisa. Nada mudou ainda.
guard-changes-download = Baixar da internet
guard-changes-download-none = Nada
guard-changes-permissions = Aprovação do sistema
guard-changes-one-prompt = Seu sistema pergunta uma vez
guard-changes-no-prompt = Não é preciso
guard-changes-restore-point = Ponto de restauração do Windows
guard-changes-restore-point-first = Pedido primeiro
guard-changes-restore-point-none = Não usado
guard-changes-install =
    { $count ->
        [one] Para instalar: { $count } app
        [many] Para instalar: { $count } de apps
       *[other] Para instalar: { $count } apps
    }
guard-changes-merge =
    { $count ->
        [one] Adicionado ao lado do que já está aqui: { $count } item
        [many] Adicionados ao lado do que já está aqui: { $count } de itens
       *[other] Adicionados ao lado do que já está aqui: { $count } itens
    }
guard-changes-replace =
    { $count ->
        [one] Para substituir, mantendo a versão antiga: { $count } item
        [many] Para substituir, mantendo as versões antigas: { $count } de itens
       *[other] Para substituir, mantendo as versões antigas: { $count } itens
    }
guard-changes-leave-alone =
    { $count ->
        [one] Deixado como está: { $count } item
        [many] Deixados como estão: { $count } de itens
       *[other] Deixados como estão: { $count } itens
    }
guard-changes-space = Espaço em cada disco no momento de maior uso
guard-changes-why-prompt = Por que seu sistema vai perguntar
guard-changes-approve = Aprovar estas mudanças
guard-undo-legend-can = O PCTwin pode desfazer.
guard-undo-legend-keeps = O PCTwin mantém. O desfazer não mexe.
guard-undo-legend-you = Precisa de você. Só o serviço ou o criador pode desfazer.
guard-undo-can-reverse = O PCTwin pode desfazer
guard-undo-can-remove-app = O PCTwin pode remover, mantendo o seu trabalho
guard-undo-must-keep = Mantido: já estava aqui
guard-undo-needs-you-service = Precisa de você: só o serviço online pode desfazer
guard-undo-needs-you-maker = Precisa de você: logins e licenças são do criador
guard-drive-needs = Precisa de { $needed }. { $free } livres.
guard-drive-needs-free-unknown = Precisa de { $needed }. O disco não informa quanto há livre.
guard-drive-fits = Cabe
guard-drive-short = Faltam { $short }
guard-drive-unknown = Não se sabe
guard-permission-read-other-people = Para ler os arquivos de outras pessoas no notebook antigo
guard-permission-make-accounts = Para criar as contas delas neste notebook
guard-permission-shared-folders = Para colocar coisas em pastas compartilhadas que a sua conta não pode alterar
guard-permission-install-for-everyone = Para instalar apps para todos neste notebook
guard-permission-restore-point = Para criar primeiro um ponto de restauração do Windows

## X06 e H10 algo mudou depois da aprovação

guard-changed-title =
    { $count ->
        [one] { $count } coisa mudou depois que você aprovou
        [many] { $count } de coisas mudaram depois que você aprovou
       *[other] { $count } coisas mudaram depois que você aprovou
    }
guard-changed-body = Só estas precisam de um novo olhar. Todo o resto continua aprovado e segue em frente.
guard-changed-changed-on-old-laptop = Mudou no notebook antigo.
guard-changed-app-updated = O app se atualizou sozinho.
guard-changed-destination-changed = O lugar para onde vai mudou.
guard-changed-new-scope = É novo: outra pessoa ou outro notebook entrou na mudança.
guard-changed-permission-ended = A aprovação para isso acabou.
guard-changed-leave-out = Deixar estas de fora
guard-changed-approve = Aprovar estas de novo

## M31, M05 espaço no notebook novo

guard-space-title = Espaço no notebook novo
guard-space-title-short = O notebook novo está com pouco espaço
guard-space-body = O PCTwin contou tudo o que chega a cada disco, inclusive o espaço para desfazer. Nada do que você escolheu é descartado para caber.
guard-space-proposal = O que o PCTwin sugere para o que não cabe
guard-keep-off-leave-in-cloud = Deixados na nuvem deles, onde estão seguros
guard-keep-off-offload-drive = Colocados no seu outro disco
guard-keep-off-leave-on-old-laptop = Deixados no notebook antigo. Guarde-o até que sejam movidos.
guard-offload-title = Seu outro disco
guard-offload-connected = Conectado
guard-offload-not-connected = Não conectado
guard-offload-approved = Você escolheu este disco. Os arquivos nele não estão no notebook novo: conecte o disco para abri-los.
guard-offload-ask = O PCTwin pode colocar algumas coisas neste disco. Ele precisa do seu sim para este disco em particular.
guard-offload-use = Usar este disco
guard-space-approve = Aprovar este plano
guard-safe-in = Seguro em { $provider }
guard-safe-in-cloud = Seguro na sua nuvem
guard-safe-in-detail =
    { $count ->
        [one] { $count } arquivo está guardado só online. Ele volta quando você entrar na sua conta no notebook novo.
        [many] { $count } de arquivos estão guardados só online. Eles voltam quando você entrar na sua conta no notebook novo.
       *[other] { $count } arquivos estão guardados só online. Eles voltam quando você entrar na sua conta no notebook novo.
    }
guard-your-cloud = sua nuvem

## M09 manter uma cópia dos arquivos que estão só online

guard-local-title = Manter uma cópia dos seus arquivos que estão só online?
guard-local-body =
    { $count ->
        [one] { $count } arquivo em { $provider } está guardado só online. Você também pode manter uma cópia dele no notebook novo.
        [many] { $count } de arquivos em { $provider } estão guardados só online. Você também pode manter uma cópia deles no notebook novo.
       *[other] { $count } arquivos em { $provider } estão guardados só online. Você também pode manter uma cópia deles no notebook novo.
    }
guard-local-download = Para baixar no notebook antigo
guard-local-old-space = Espaço no notebook antigo
guard-local-how = O PCTwin baixa alguns de cada vez, envia e depois os deixa só online de novo no notebook antigo.
guard-local-paused-metered = Pausado: esta conexão pode cobrar pelos dados. Continua sozinho em uma conexão gratuita, ou agora mesmo se você permitir.
guard-local-paused-no-space = Pausado: o notebook antigo ficou sem espaço para os próximos arquivos. O resto continua seguro em { $provider }.
guard-local-done = Pronto. As cópias estão no notebook novo.
guard-local-keep = Manter uma cópia
guard-local-leave = Deixar em { $provider }
guard-local-allow-metered = Permitir nesta conexão

## M07 quem vai para onde

guard-owners-title = Quem vai para onde
guard-owners-body = As coisas de cada pessoa vão para a conta dela neste notebook. Nomes iguais nunca decidem isso sozinhos.
guard-owners-from = De { $laptop }
guard-owners-to-account = Para a conta { $account } neste notebook
guard-owners-new-account = Para uma conta nova neste notebook
guard-owners-own-folder = As próprias pastas “De { $laptop }” dela, porque os nomes se repetem
guard-owners-usual-folders = As pastas de sempre dela
guard-owners-approve = Aprovar

## X12 o que não terminou

guard-report-title-partial = Quase tudo foi movido. Algumas coisas não.
guard-report-title-done = Tudo nesta parte foi movido.
guard-report-body-partial = Tudo abaixo ainda está no notebook antigo. Tente cada coisa de novo quando quiser. O que já foi movido continua lá.
guard-report-body-done = Não resta nada para tentar de novo.
guard-line-app = O app { $name }
guard-line-items =
    { $count ->
        [one] { $count } item
        [many] { $count } de itens
       *[other] { $count } itens
    }
guard-line-settings =
    { $count ->
        [one] { $count } configuração
        [many] { $count } de configurações
       *[other] { $count } configurações
    }
guard-line-why-failed = De { $laptop }: não terminou.
guard-line-why-laptop-not-connected = De { $laptop }: esse notebook não está conectado. Conecte-o ao mesmo Wi-Fi e mantenha-o ligado.
guard-line-why-waits-for-another = De { $laptop }: espera por outra etapa que não terminou. Resolva essa primeiro.
guard-line-why-not-started = De { $laptop }: a mudança parou antes de começar.

## M25, H10 a aprovação do sistema, e o ponto de restauração

guard-permission-title-explain = O PCTwin precisa de uma aprovação do seu sistema
guard-permission-title-waiting-for-system = Responda à pergunta do seu sistema
guard-permission-title-denied = Você disse não
guard-permission-title-closed-without-answer = A pergunta foi fechada sem resposta
guard-permission-title-given = Aprovado para esta mudança
guard-permission-title-ended-by-sleep = A aprovação acabou quando o notebook entrou em repouso
guard-permission-title-ended-other-session = A aprovação acabou quando outra pessoa entrou
guard-permission-body-explain = Seu sistema pergunta uma vez para a mudança toda. Só as etapas listadas abaixo usam essa aprovação, e ela acaba com a mudança.
guard-permission-body-waiting-for-system = A janela do seu próprio sistema está aberta. Digite sua senha lá, se ela pedir.
guard-permission-body-denied = Só as etapas que precisavam dela ficam de fora. Todo o resto continua.
guard-permission-body-closed-without-answer = Fechar não é um sim, então nada que precisa dela foi feito. Peça de novo ou continue sem ela.
guard-permission-body-given = O PCTwin a usa só para as etapas listadas abaixo, e ela acaba com esta mudança.
guard-permission-body-ended-by-sleep = Nada que precisa dela foi feito depois disso. Peça de novo para continuar.
guard-permission-body-ended-other-session = Nada que precisa dela foi feito depois disso. Peça de novo quando voltar para a sua conta.
guard-permission-never-password = O PCTwin nunca pede a sua senha e nunca a vê. Só o seu sistema vê.
guard-permission-not-now = Continuar sem ela
guard-permission-ask = Continuar para o seu sistema
guard-permission-ask-again = Pedir de novo
guard-restore-title = Ponto de restauração do Windows
guard-restore-made = Criado às { $time }.
guard-restore-earlier-exists = O Windows já tem um de { $time }, então não criou outro.
guard-restore-not-chosen = Não criado: você escolheu não criar.
guard-restore-not-made-not-windows = Não criado: pontos de restauração só existem no Windows.
guard-restore-not-made-protection-off = Não criado: System Protection está desativada para este disco. Você pode ativá-la em System Properties.
guard-restore-not-made-needs-admin = Não criado: precisa da aprovação do seu sistema.
guard-restore-not-made-unconfirmed = Não criado: o Windows disse sim, mas nenhum ponto de restauração foi encontrado.
guard-restore-not-made-failed = Não criado: o Windows não conseguiu criar um.

## T01 a T04 resgate de um notebook que não liga

guard-rescue-title-intro = Salve o que importa de um notebook que não liga
guard-rescue-title-pick-drive = Escolha o disco para resgatar
guard-rescue-title-locked = Este disco está bloqueado
guard-rescue-title-no-system = Este não é o disco de um notebook
guard-rescue-title-salvage = Isto é o que foi possível ler
guard-rescue-body-intro = Tire o disco do notebook antigo e conecte-o a este com um adaptador ou um estojo para disco.
guard-rescue-body-pick-drive = Escolha o disco do notebook antigo. O disco para onde as coisas vão não pode ser o que você está resgatando.
guard-rescue-body-locked = Desbloqueie-o primeiro no seu sistema. Depois, verifique de novo.
guard-rescue-body-no-system = Este disco não tem um sistema de notebook nem pastas pessoais. Se for um disco USB comum, use-o no notebook novo.
guard-rescue-body-salvage = Confira a lista e use o que foi possível ler.
guard-rescue-can = O que um resgate pode fazer
guard-rescue-can-body = Ler seus arquivos, e as pessoas e configurações encontradas no disco, de um disco que ainda funciona.
guard-rescue-cannot = O que ele não pode fazer
guard-rescue-cannot-body = Consertar o disco, ler partes danificadas ou abrir um disco cuja chave você não tem.
guard-rescue-read-only-title = O disco só é lido
guard-rescue-read-only-promise = O PCTwin só lê o disco. Nada nele é alterado, consertado ou formatado.
guard-rescue-system-read-only = Seu sistema abriu o disco só para leitura, então nada nele pode mudar.
guard-rescue-pctwin-reads-only = O PCTwin só leu o disco. Seu sistema o tinha aberto para alterações, então outros apps ainda podiam mudá-lo.
guard-rescue-refused-target = Esse disco é para onde as coisas vão, então ele não pode ser resgatado também. Escolha outro disco.
guard-rescue-drive-detail = { $size }, { $fs }
guard-rescue-drive-is-target = As coisas da mudança vão para este disco.
guard-rescue-drive-cannot = Não pode ser resgatado
guard-rescue-choose = Resgatar este disco
guard-rescue-bit-locker-step-1 = Abra o disco no File Explorer.
guard-rescue-bit-locker-step-2 = Digite a chave de recuperação quando o Windows pedir. Ela costuma estar salva na conta Microsoft do dono, em aka.ms/myrecoverykey.
guard-rescue-bit-locker-step-3 = Volte aqui e escolha Verificar de novo.
guard-rescue-file-vault-step-1 = Desbloqueie-o em um Mac com a senha de login do dono ou a chave de recuperação.
guard-rescue-file-vault-step-2 = Volte aqui e escolha Verificar de novo.
guard-rescue-apple-chip-step-1 = Este Mac mantém o disco bloqueado só para ele, então tirar o disco não funciona.
guard-rescue-apple-chip-step-2 = Se o Mac ainda liga em Recovery, use Share Disk com um cabo. Se ele não liga de jeito nenhum, só uma assistência técnica da Apple pode ajudar.
guard-rescue-never-key = O PCTwin nunca pede a chave e nunca a guarda.
guard-rescue-readable =
    { $count ->
        [one] { $n } item pode ser lido.
        [many] { $n } de itens podem ser lidos.
       *[other] { $n } itens podem ser lidos.
    }
guard-rescue-unreadable =
    { $count ->
        [one] { $count } item não pôde ser lido
        [many] { $count } de itens não puderam ser lidos
       *[other] { $count } itens não puderam ser lidos
    }
guard-rescue-look = Procurar o disco
guard-rescue-check-again = Verificar de novo
guard-rescue-use = Usar o que foi possível ler

## R05, R06 desfazer

guard-undo-preview-title = O que o desfazer vai fazer
guard-undo-preview-body = O PCTwin olhou cada mudança agora há pouco. Tudo o que você alterou depois da mudança é mantido. Nada acontece até você escolher Desfazer.
guard-undo-preview-which = Isto desfaz a transferência que terminou em { DATETIME($when, year: "numeric", month: "long", day: "numeric", hour: "numeric", minute: "2-digit") }. Suas outras transferências não mudam.
guard-undo-preview-which-earlier = Isto desfaz a transferência feita com uma versão anterior do PCTwin. Suas outras transferências não mudam.
guard-undo-plan-put-back = Devolvido como era
guard-undo-plan-remove-app = App removido, seu trabalho mantido
guard-undo-plan-keep-changed = Mantido: você alterou
guard-undo-plan-elsewhere = Precisa de você: feito em outro lugar
guard-undo-plan-leave-alone = Deixado como está: já estava aqui
guard-undo-keep = Manter tudo como está
guard-undo-run = Desfazer
guard-undo-running-title = Desfazendo as mudanças
guard-undo-done-title = O que o desfazer fez
guard-undo-result-restored = Restaurado
guard-undo-result-removed = Removido
guard-undo-result-as-before = Já estava como antes
guard-undo-result-kept-changed = Mantido porque você alterou
guard-undo-result-kept-other = Mantido, veja Mais detalhes
guard-undo-result-kept-at = Mantido como { $name }
guard-undo-result-done-elsewhere = Feito em outro lugar: só o serviço ou o criador pode desfazer
guard-undo-result-not-done-yet = Ainda não foi feito. O desfazer tenta de novo da próxima vez.

## The full move: apps, settings and app data (28 October 2026)

fullmove-plan-title = Apps, configurações e dados de apps
fullmove-plan-body = O PCTwin olhou os apps, as configurações e os dados de apps do seu notebook antigo. Isto é o que ele fará neste notebook depois dos seus arquivos. Nada muda até você iniciar a mudança.
fullmove-group-install = Apps que o PCTwin vai instalar: { $count }
fullmove-group-settings = Configurações de apps que o PCTwin vai copiar: { $count }
fullmove-group-needs = Coisas que você mesmo vai fazer: { $count }
fullmove-group-here = Apps que já estão neste notebook: { $count }
fullmove-settings-count = Configurações: { $count }.
fullmove-settings-merge = Elas são adicionadas. Nada neste notebook é substituído.
fullmove-settings-replace = Algumas configurações deste notebook são substituídas. O desfazer pode trazê-las de volta.
fullmove-app-install = O PCTwin instala a partir do criador e verifica.
fullmove-app-here = Ele já está neste notebook, então o PCTwin o deixa como está.
fullmove-step-copy = Copiado, com desfazer.
fullmove-step-export-first = No notebook antigo, use a exportação do próprio app. Depois importe no app deste notebook.
fullmove-step-sign-in = Entre de novo neste notebook.
fullmove-step-relink = Configure de novo neste notebook ou vincule de novo pelo app.
fullmove-step-not-yet = O PCTwin ainda não move os dados deste app. Eles continuam seguros no seu notebook antigo.
fullmove-step-look-at-it = O PCTwin não conseguiu saber o que é isto. Veja no seu notebook antigo.
fullmove-special-not-yet = Dados de apps como e-mail, navegadores e jogos
fullmove-special-finish = O PCTwin não moveu isto. Continua seguro no seu notebook antigo.
fullmove-more = E mais { $count }.
fullmove-not-moved-title = Configurações que o PCTwin deixa no notebook antigo
fullmove-not-moved-private = Configurações privadas e de segurança, e logins, que nunca são copiados: { $count }
fullmove-not-moved-could-run = Configurações que iniciariam ou abririam algo, guardadas por segurança: { $count }
fullmove-not-moved-old-laptop-only = Configurações que só servem no notebook antigo: { $count }
fullmove-not-moved-system-own = Configurações do próprio sistema que o PCTwin ainda não move: { $count }
fullmove-not-moved-not-read = Configurações encontradas, mas não lidas: { $count }
fullmove-special-not-moved = Arquivos de apps deixados para trás, como caches: { $count }
fullmove-prompt = Este notebook vai pedir uma vez a permissão de um administrador.
fullmove-prompt-restore = Este notebook vai pedir uma vez a permissão de um administrador e criar um ponto de restauração antes da primeira mudança.
fullmove-rebuilding-title = Preparando seus apps e configurações
fullmove-rebuilding-body = Seus arquivos estão no lugar. O PCTwin está preparando o resto. Mantenha este notebook ligado.
fullmove-rebuilding-permission = Pedindo permissão a este notebook
fullmove-rebuilding-apps = Apps: { $done } de { $total }
fullmove-rebuilding-settings = Configurações: { $done } de { $total }
fullmove-outcome-title = Apps, configurações e dados de apps
fullmove-outcome-undo = Desfazer configurações de apps
fullmove-outcome-stopped = O PCTwin parou de mudar configurações no meio, porque não conseguiu anotar a próxima mudança. Nada depois desse ponto foi mudado.
fullmove-restore-made = Um ponto de restauração foi criado antes das mudanças.
fullmove-restore-earlier-exists = Nenhum ponto de restauração novo foi criado, porque este notebook já tinha um de hoje.
fullmove-restore-not-made = Nenhum ponto de restauração foi criado. O desfazer do PCTwin continua funcionando.
fullmove-restore-not-windows = Este notebook não tem pontos de restauração. O desfazer do PCTwin continua funcionando.
guard-undo-remove-yourself = Remova você mesmo, se quiser
guard-undo-plan-remove-yourself = Você pode removê-lo nas configurações do notebook
guard-undo-result-remove-yourself = Deixado para você remover nas configurações do notebook
guard-undo-close = Pronto
move-wait-appdata-title = Olhando os dados dos seus apps.
move-wait-appdata-body = Seus arquivos foram encontrados. O PCTwin ainda está olhando os dados que seus apps guardam, como e-mail e navegadores. Isso leva no máximo dez minutos. Nada neste notebook é mudado.
fullmove-special-not-checked = Dados de apps que o PCTwin não conseguiu verificar por completo
fullmove-group-appdata = Dados de apps que o PCTwin vai copiar: { $count }
fullmove-group-secrets = Arquivos secretos que você pode escolher trazer: { $count }
fullmove-secrets-body = Eles guardam senhas ou chaves. O PCTwin só traz um se você mesmo marcar. Sem marcar, ele fica no seu notebook antigo.
fullmove-secret-bring = Trazer { $name }
fullmove-secret-hint = Só você poderá abri-lo neste notebook. O desfazer pode removê-lo de novo.
fullmove-appdata-title = Copiando os dados dos seus apps
fullmove-appdata-progress = Copiando dados de apps: { $done } de { $total } apps
fullmove-appdata-waiting = Esperando o notebook antigo: feche { $apps } lá.
fullmove-close-title = Feche estes apps para trazer os dados deles
fullmove-close-body = Estes apps ainda estão abertos neste notebook, então o PCTwin não consegue copiar os dados deles com segurança. Feche-os e escolha Tentar de novo. Se continuar sem eles, os dados ficam aqui.
fullmove-close-again = Tentar de novo
fullmove-close-without = Continuar sem eles
fullmove-extras-refused = Não foi possível ler aqui as configurações e os dados de apps do notebook antigo, então nada foi movido. Eles continuam no seu notebook antigo.
fullmove-app-from = O PCTwin instala o pacote do winget { $id } do criador, depois de verificar que é este app e o mesmo criador.
fullmove-here-older = Há uma versão mais antiga ({ $version }) neste notebook. O PCTwin a deixa como está.
fullmove-step-sites = Os logins salvos em sites continuam presos ao notebook antigo. Entre de novo nesses sites neste notebook.
fullmove-links = Atalhos para outros lugares entre seus arquivos, que o PCTwin não segue nem transfere: { $count }
fullmove-note-sign-in = Abra o app e entre na sua conta.
fullmove-note-licence-key = Abra o app e digite sua chave de licença, ou entre na conta que tem a sua licença.
fullmove-note-registration = Seu notebook antigo mostrava o app registrado em nome de { $who }.
fullmove-note-product-id = O ID do produto no seu notebook antigo era { $id }.
fullmove-note-extras = Outros programas vieram junto: { $apps }. Remova-os nas configurações do notebook se não os quiser.
fullmove-data-lost = A conexão caiu antes de estes dados chegarem. Eles continuam seguros no seu notebook antigo. Comece uma nova transferência para trazê-los.
fullmove-data-did-not-arrive = Estes dados não chegaram. Eles continuam seguros no seu notebook antigo. Comece uma nova transferência para trazê-los.
fullmove-data-app-open = O app estava aberto no seu notebook antigo, então os dados dele não foram copiados. Feche o app lá e comece uma nova transferência para trazê-los.
fullmove-data-changing = Estes dados mudavam no seu notebook antigo enquanto o PCTwin os lia. Feche o app lá e comece uma nova transferência para trazê-los.
fullmove-data-unreadable = O PCTwin não conseguiu ler estes dados no seu notebook antigo. Eles continuam lá.
fullmove-data-not-read = A transferência parou antes de estes dados serem lidos. Eles continuam seguros no seu notebook antigo.
fullmove-data-not-whole = Nem todos estes dados chegaram, então o PCTwin não colocou nenhum no lugar. Eles continuam seguros no seu notebook antigo.
fullmove-data-not-landed = Estes dados chegaram, mas o PCTwin não conseguiu colocá-los no lugar neste notebook. Eles continuam seguros no seu notebook antigo.
fullmove-data-beside = O app já tinha a própria pasta neste notebook, então o PCTwin colocou a cópia do notebook antigo ao lado, em { $folder }. O app continua usando a própria pasta. Para usar a cópia antiga, feche o app e troque as duas pastas.
fullmove-data-placed = Os dados estão em { $folder }, mas o app precisa de um passo antes de poder usá-los. Abra o app e confira.
fullmove-outcome-here-title = Apps que já estão neste notebook: { $count }
fullmove-outcome-here = Já está neste notebook, deixado como está.
fullmove-outcome-here-older = Há uma versão mais antiga ({ $version }) neste notebook. O PCTwin a deixou como está. Atualize pelo criador se quiser a mais nova.
fullmove-outcome-here-badge = Deixado como está
fullmove-outcome-fresh = Os apps que o PCTwin instalou começam do zero. Abra cada um e entre na conta se ele pedir.
progress-may-sleep = Este notebook pode entrar em suspensão durante a transferência. Mantenha-o na tomada e aberto.
