# Device labels: word order and gender agreement (fr, es, pt, ar, hi, zh-CN)

Each laptop is named while pairing by a colour ID plus an animal ID (12 colours x 100 animals), shown as words, e.g. English "Blue Fox".
`labelText()` in `app/src/pairing.ts` builds it: it looks up `label-gender-<animal>` (if the file has it; otherwise gender is ""),
then renders `label-colour-<colour>` with `{ $gender }`, `label-animal-<animal>`, and puts both into `label-picked`.

## The mechanism (no new dependency, plain Fluent)

1. A per-animal message in the TRANSLATED files only (never in en): its whole value is exactly `masculine` or `feminine`
   (lowercase ASCII, not translated, no extra text). Put all 100 right after the last `label-animal-*` line.

       label-gender-fox = masculine
       label-gender-hen = feminine        (id must equal the animal ID; use the table below)

2. A colour message that varies by gender selects on `$gender`; the default variant is always `*[masculine]`
   (used when `$gender` is "" or the animal has no gender message). Colours that do not vary are plain text, with NO selector.

       label-colour-blue =
           { $gender ->
               [feminine] bleue
              *[masculine] bleu
           }

   Variant keys are exactly `feminine` and `masculine`. Never add other keys. Never put `$gender` in label-picked or animal messages.

3. `label-picked` sets the order (colour and animal are already isolated by Fluent; do not add direction marks):

       fr, es, pt, ar:  label-picked = { $animal } { $colour }      (animal first)
       hi:              label-picked = { $colour } { $animal }      (colour first, same as English)
       zh-CN:           label-picked = { $colour }{ $animal }       (colour first, no space)

4. Case: fr, es, pt: capitalise the animal noun (it opens the label), colour in lowercase mid-label ("Renard bleu"). ar, hi, zh have no case.
   Never write the colour with a capital in fr/es/pt: it is never shown alone. Animal noun: singular, no article.

5. Checks for the Localizer gate: `$gender` appears only inside `label-colour-*` (it is not in en); `label-gender-*` ids exist only in
   fr/es/pt/ar/hi (100 each, all with value `masculine` or `feminine`); `[feminine]/*[masculine]` selectors are not CLDR plural selectors
   and must be exempt from the plural-category gate. Calling `text("label-colour-blue")` without `gender` (as the tests do) reports an
   unknown-variable error but still returns the default variant: tests should pass `{ gender: "" }`.

## Colours: forms per language (masc / fem; one form = invariable, write plain text, no selector)

    fr  red rouge | orange orange | yellow jaune | green vert/verte | blue bleu/bleue | purple violet/violette
        pink rose | brown marron | black noir/noire | white blanc/blanche | grey gris/grise | gold doré/dorée
        (selectors: green, blue, purple, black, white, grey, gold)
    es  red rojo/roja | orange naranja | yellow amarillo/amarilla | green verde | blue azul | purple violeta
        pink rosa | brown marrón | black negro/negra | white blanco/blanca | grey gris | gold dorado/dorada
        (selectors: red, yellow, black, white, gold)
    pt  red vermelho/vermelha | orange laranja | yellow amarelo/amarela | green verde | blue azul | purple roxo/roxa
        pink rosa | brown marrom | black preto/preta | white branco/branca | grey cinza | gold dourado/dourada
        (selectors: red, yellow, purple, black, white, gold)
    ar  ALL twelve vary:
        red أحمر/حمراء | orange برتقالي/برتقالية | yellow أصفر/صفراء | green أخضر/خضراء | blue أزرق/زرقاء | purple بنفسجي/بنفسجية
        pink وردي/وردية | brown بني/بنية | black أسود/سوداء | white أبيض/بيضاء | grey رمادي/رمادية | gold ذهبي/ذهبية
    hi  red लाल | orange नारंगी | yellow पीला/पीली | green हरा/हरी | blue नीला/नीली | purple बैंगनी
        pink गुलाबी | brown भूरा/भूरी | black काला/काली | white सफ़ेद | grey स्लेटी | gold सुनहरा/सुनहरी
        (selectors: yellow, green, blue, brown, black, gold)
    zh  no gender, no selectors: 红色 橙色 黄色 绿色 蓝色 紫色 粉色 棕色 黑色 白色 灰色 金色
        (keep "色" so a one-character colour never fuses with the animal, e.g. 黄色 + 蜂 vs 黄蜂)

Notes: fr "pink/brown" use the invariable names (rose, marron) on purpose; es/pt "brown" uses marrón/marrom (invariable) rather than café or castanho.
There is no panther in the list, so "Panthère noire" does not occur.

## Animal gender tables (ids in en/main.ftl order; format `id=m|f noun-assumed`)

The gender below is the grammatical gender of the noun in the third column. If the Translator picks a different noun, the gender of THAT noun wins
and must be written in `label-gender-<id>`. Nouns are suggestions; hard ones are marked * and explained under "Hard animals".

### fr (100)
    ant=f fourmi; badger=m blaireau; bat=f chauve-souris; bear=m ours; bee=f abeille; beetle=m scarabée;
    bison=m bison; butterfly=m papillon; camel=m chameau; cat=m chat; cheetah=m guépard; chicken=f poule;
    crab=m crabe; crane=f grue; crocodile=m crocodile; crow=m corbeau; deer=m cerf; dolphin=m dauphin;
    donkey=m âne; dove=f colombe; dragonfly=f libellule; duck=m canard; eagle=m aigle; elephant=m éléphant;
    falcon=m faucon; flamingo=m flamant; fox=m renard; frog=f grenouille; gazelle=f gazelle; swordfish=m espadon;
    giraffe=f girafe; goat=f chèvre; goose=f oie; gorilla=m gorille; hamster=m hamster; hawk=f buse*;
    hedgehog=m hérisson; armadillo=m tatou; hippo=m hippopotame; horse=m cheval; hummingbird=m colibri; jaguar=m jaguar;
    jellyfish=f méduse; kangaroo=m kangourou; kingfisher=m martin-pêcheur; koala=m koala; ladybird=f coccinelle; lemur=m lémurien*;
    leopard=m léopard; lion=m lion; lizard=m lézard; llama=m lama; lobster=m homard; lynx=m lynx;
    macaw=m ara; meerkat=m suricate; mole=f taupe; moose=m élan; pufferfish=m poisson-globe; mouse=f souris;
    octopus=f pieuvre; ostrich=f autruche; otter=f loutre; owl=f chouette; panda=m panda; kiwi=m kiwi*;
    parrot=m perroquet; peacock=m paon; pelican=m pélican; penguin=m manchot; puffin=m macareux; rabbit=m lapin;
    raccoon=m raton laveur; rhino=m rhinocéros; robin=m rouge-gorge; salmon=m saumon; seahorse=m hippocampe; seal=m phoque;
    shark=m requin; sheep=m mouton; sloth=m paresseux; snail=m escargot; sparrow=m moineau; squirrel=m écureuil;
    starfish=f étoile de mer; stork=f cigogne; swan=m cygne; tiger=m tigre; snake=m serpent; toucan=m toucan;
    turtle=f tortue; walrus=m morse; whale=f baleine; wolf=m loup; woodpecker=m pivert; yak=m yak;
    zebra=m zèbre; dog=m chien; cow=f vache; beaver=m castor

### es (100, neutral international)
    ant=f hormiga; badger=m tejón; bat=m murciélago; bear=m oso; bee=f abeja; beetle=m escarabajo;
    bison=m bisonte; butterfly=f mariposa; camel=m camello; cat=m gato; cheetah=m guepardo; chicken=f gallina;
    crab=m cangrejo; crane=f grulla; crocodile=m cocodrilo; crow=m cuervo; deer=m ciervo*; dolphin=m delfín;
    donkey=m burro; dove=f paloma; dragonfly=f libélula; duck=m pato; eagle=f águila; elephant=m elefante;
    falcon=m halcón; flamingo=m flamenco; fox=m zorro; frog=f rana; gazelle=f gacela; swordfish=m pez espada;
    giraffe=f jirafa; goat=f cabra; goose=m ganso; gorilla=m gorila; hamster=m hámster; hawk=m gavilán;
    hedgehog=m erizo; armadillo=m armadillo; hippo=m hipopótamo; horse=m caballo; hummingbird=m colibrí; jaguar=m jaguar;
    jellyfish=f medusa; kangaroo=m canguro; kingfisher=m martín pescador; koala=m koala; ladybird=f mariquita*; lemur=m lémur;
    leopard=m leopardo; lion=m león; lizard=m lagarto; llama=f llama; lobster=f langosta; lynx=m lince;
    macaw=m guacamayo; meerkat=f suricata; mole=m topo; moose=m alce; pufferfish=m pez globo; mouse=m ratón;
    octopus=m pulpo; ostrich=m avestruz; otter=f nutria; owl=m búho; panda=m panda; kiwi=m kiwi*;
    parrot=m loro; peacock=m pavo real; pelican=m pelícano; penguin=m pingüino; puffin=m frailecillo*; rabbit=m conejo;
    raccoon=m mapache; rhino=m rinoceronte; robin=m petirrojo; salmon=m salmón; seahorse=m caballito de mar; seal=f foca;
    shark=m tiburón; sheep=f oveja; sloth=m perezoso; snail=m caracol; sparrow=m gorrión; squirrel=f ardilla;
    starfish=f estrella de mar; stork=f cigüeña; swan=m cisne; tiger=m tigre; snake=f serpiente; toucan=m tucán;
    turtle=f tortuga; walrus=f morsa; whale=f ballena; wolf=m lobo; woodpecker=m pájaro carpintero; yak=m yak;
    zebra=f cebra; dog=m perro; cow=f vaca; beaver=m castor
    (note: águila and llama are feminine nouns that take "el/una" oddly; the adjective is feminine: "Águila roja", "Llama negra")

### pt (100, Brazilian)
    ant=f formiga; badger=m texugo; bat=m morcego; bear=m urso; bee=f abelha; beetle=m besouro;
    bison=m bisão; butterfly=f borboleta; camel=m camelo; cat=m gato; cheetah=m guepardo; chicken=f galinha;
    crab=m caranguejo; crane=m grou; crocodile=m crocodilo; crow=m corvo; deer=m cervo; dolphin=m golfinho;
    donkey=m burro; dove=f pomba; dragonfly=f libélula; duck=m pato; eagle=f águia; elephant=m elefante;
    falcon=m falcão; flamingo=m flamingo; fox=f raposa; frog=m sapo; gazelle=f gazela; swordfish=m peixe-espada;
    giraffe=f girafa; goat=f cabra; goose=m ganso; gorilla=m gorila; hamster=m hamster; hawk=m gavião;
    hedgehog=m ouriço; armadillo=m tatu; hippo=m hipopótamo; horse=m cavalo; hummingbird=m beija-flor; jaguar=m jaguar;
    jellyfish=f água-viva; kangaroo=m canguru; kingfisher=m martim-pescador; koala=m coala; ladybird=f joaninha; lemur=m lêmure;
    leopard=m leopardo; lion=m leão; lizard=m lagarto; llama=f lhama; lobster=f lagosta; lynx=m lince;
    macaw=f arara; meerkat=m suricato; mole=f toupeira; moose=m alce; pufferfish=m baiacu; mouse=m rato;
    octopus=m polvo; ostrich=m avestruz; otter=f lontra; owl=f coruja; panda=m panda; kiwi=m quivi*;
    parrot=m papagaio; peacock=m pavão; pelican=m pelicano; penguin=m pinguim; puffin=m papagaio-do-mar*; rabbit=m coelho;
    raccoon=m guaxinim; rhino=m rinoceronte; robin=m pisco*; salmon=m salmão; seahorse=m cavalo-marinho; seal=f foca;
    shark=m tubarão; sheep=f ovelha; sloth=f preguiça; snail=m caracol; sparrow=m pardal; squirrel=m esquilo;
    starfish=f estrela-do-mar; stork=f cegonha; swan=m cisne; tiger=m tigre; snake=f cobra; toucan=m tucano;
    turtle=f tartaruga; walrus=f morsa; whale=f baleia; wolf=m lobo; woodpecker=m pica-pau; yak=m iaque;
    zebra=f zebra; dog=m cachorro; cow=f vaca; beaver=m castor

### ar (100; noun then adjective, adjective agrees; loanwords ending in alif count as masculine)
    ant=f نملة; badger=m غرير; bat=m خفاش; bear=m دب; bee=f نحلة; beetle=f خنفساء;
    bison=m بيسون; butterfly=f فراشة; camel=m جمل; cat=f قطة; cheetah=m شيتا; chicken=f دجاجة;
    crab=m سلطعون; crane=m كركي; crocodile=m تمساح; crow=m غراب; deer=m أيل; dolphin=m دلفين;
    donkey=m حمار; dove=f حمامة; dragonfly=m يعسوب; duck=f بطة; eagle=m نسر; elephant=m فيل;
    falcon=m صقر; flamingo=m فلامنغو; fox=m ثعلب; frog=m ضفدع; gazelle=m غزال; swordfish=m أبو سيف*;
    giraffe=f زرافة; goat=f عنزة; goose=f إوزة; gorilla=m غوريلا; hamster=m همستر; hawk=m باز;
    hedgehog=m قنفذ; armadillo=m أرماديلو*; hippo=m فرس النهر*; horse=m حصان; hummingbird=m طائر طنان; jaguar=m يغوار;
    jellyfish=m قنديل البحر; kangaroo=m كنغر; kingfisher=m رفراف; koala=m كوالا; ladybird=f دعسوقة; lemur=m ليمور;
    leopard=m فهد; lion=m أسد; lizard=f سحلية; llama=m لاما; lobster=m كركند; lynx=m وشق;
    macaw=m ماكاو; meerkat=m ميركات; mole=m خلد; moose=m موظ; pufferfish=f سمكة منتفخة; mouse=m فأر;
    octopus=m أخطبوط; ostrich=f نعامة; otter=f قضاعة; owl=f بومة; panda=m باندا; kiwi=m كيوي;
    parrot=m ببغاء; peacock=m طاووس; pelican=m بجع*; penguin=m بطريق; puffin=m ببغاء البحر*; rabbit=m أرنب;
    raccoon=m راكون; rhino=m وحيد القرن; robin=m أبو الحناء; salmon=m سلمون; seahorse=m حصان البحر; seal=f فقمة;
    shark=m قرش; sheep=m خروف; sloth=m كسلان; snail=m حلزون; sparrow=m عصفور; squirrel=m سنجاب;
    starfish=f نجمة البحر; stork=m لقلق; swan=f بجعة*; tiger=m نمر; snake=m ثعبان; toucan=m توكان;
    turtle=f سلحفاة; walrus=m فظ; whale=m حوت; wolf=m ذئب; woodpecker=m نقار الخشب; yak=m ياك;
    zebra=m حمار وحشي; dog=m كلب; cow=f بقرة; beaver=m قندس

### hi (100; colour comes first and agrees with the animal; loanwords default to masculine unless they end in a feminine pattern or mean a fish)
    ant=f चींटी; badger=m बिज्जू; bat=m चमगादड़; bear=m भालू; bee=f मधुमक्खी; beetle=m गुबरैला;
    bison=m बाइसन; butterfly=f तितली; camel=m ऊँट; cat=f बिल्ली; cheetah=m चीता; chicken=f मुर्गी;
    crab=m केकड़ा; crane=m सारस; crocodile=m मगरमच्छ; crow=m कौआ; deer=m हिरन; dolphin=f डॉल्फ़िन;
    donkey=m गधा; dove=f फ़ाख़्ता; dragonfly=f ड्रैगनफ़्लाई*; duck=f बत्तख़; eagle=m गरुड़; elephant=m हाथी;
    falcon=m शाहीन; flamingo=m फ़्लेमिंगो; fox=f लोमड़ी; frog=m मेंढक; gazelle=m चिकारा; swordfish=f तलवार मछली;
    giraffe=m जिराफ़; goat=f बकरी; goose=m कलहंस; gorilla=m गोरिल्ला; hamster=m हैम्स्टर; hawk=m बाज़;
    hedgehog=m हेजहॉग*; armadillo=m आर्मडिलो; hippo=m दरियाई घोड़ा; horse=m घोड़ा; hummingbird=m हमिंगबर्ड; jaguar=m जैगुआर;
    jellyfish=f जेलीफ़िश; kangaroo=m कंगारू; kingfisher=m किलकिला; koala=m कोआला; ladybird=m लेडीबग*; lemur=m लीमर;
    leopard=m तेंदुआ; lion=m शेर; lizard=f छिपकली; llama=m लामा; lobster=m झींगा; lynx=m वनबिलाव;
    macaw=m मकाउ; meerkat=m मीरकैट; mole=f छछूंदर; moose=m मूस; pufferfish=f पफ़रफ़िश; mouse=m चूहा;
    octopus=m ऑक्टोपस; ostrich=m शुतुरमुर्ग; otter=m ऊदबिलाव; owl=m उल्लू; panda=m पांडा; kiwi=m कीवी;
    parrot=m तोता; peacock=m मोर; pelican=m पेलिकन; penguin=m पेंगुइन; puffin=m पफ़िन; rabbit=m ख़रगोश;
    raccoon=m रैकून; rhino=m गैंडा; robin=m रॉबिन; salmon=f सैल्मन; seahorse=m समुद्री घोड़ा; seal=f सील;
    shark=f शार्क; sheep=f भेड़; sloth=m आलसी भालू; snail=m घोंघा; sparrow=f गौरैया; squirrel=f गिलहरी;
    starfish=f तारामछली; stork=m लकलक*; swan=m हंस; tiger=m बाघ; snake=m साँप; toucan=m टूकन;
    turtle=m कछुआ; walrus=m वालरस; whale=f व्हेल; wolf=m भेड़िया; woodpecker=m कठफोड़वा; yak=m याक;
    zebra=m ज़ेबरा; dog=m कुत्ता; cow=f गाय; beaver=m बीवर

### zh-CN
No `label-gender-*` messages and no selectors. 100 plain `label-animal-*` nouns (single or two-character names, no articles or measure words).

## Hard animals (suggested common nouns; * above)
- ladybird: fr coccinelle; es mariquita (Spain; "catarina" in Mexico, understood less elsewhere); pt joaninha; ar دعسوقة; hi लेडीबग (no everyday Hindi word); zh 瓢虫.
- kiwi: the bird, not the fruit: fr kiwi; es kiwi; pt quivi; ar كيوي; hi कीवी; zh 几维鸟 (not 猕猴桃). The label pairs it with a colour, so confusion is low.
- puffin: fr macareux; es frailecillo (little known in Latin America); pt papagaio-do-mar; ar ببغاء البحر; hi पफ़िन; zh 海雀.
- meerkat: fr suricate; es suricata; pt suricato; ar ميركات; hi मीरकैट; zh 狐獴.
- lemur: fr lémurien (lémur also used); es lémur; pt lêmure; ar ليمور; hi लीमर; zh 狐猴.
- deer: es ciervo (Spain; "venado" more Latin American, both understood; keep ciervo). moose: es alce, fr élan, pt alce.
- hawk and falcon and eagle must stay distinct: fr buse / faucon / aigle; es gavilán / halcón / águila; pt gavião / falcão / águia; ar باز / صقر / نسر; hi बाज़ / शाहीन / गरुड़.
- ar: swan بجعة and pelican بجع read almost alike (a taa marbuta apart); accept, or choose pelican "البجع الأبيض" if the Terminologist wants a stronger split. armadillo (أرماديلو, transliterated), hippo (فرس النهر), puffin (ببغاء البحر), swordfish (أبو سيف) are the awkward nouns.
- ar: tiger نمر, leopard فهد, cheetah شيتا (loan), jaguar يغوار must all stay different words.
- hi: stork vs crane: crane is सारस; stork लकलक is unfamiliar to many but must not reuse सारस. Dragonfly ड्रैगनफ़्लाई and hedgehog हेजहॉग are loanwords; loanword genders in Hindi vary, the table's gender is the one to use consistently.
- fr hawk "buse" is a buzzard; "épervier" (m) is an acceptable alternative: change `hawk=m` if used.
- zh: simple, distinct nouns exist for all 100; keep tiger 老虎, leopard 豹子, cheetah 猎豹, jaguar 美洲豹 all different, and swan 天鹅, goose 鹅, duck 鸭.

## Test rule
After translating, the Localizer renders every colour x every animal (1200 labels) per language through `labelText` and checks: output non-empty, no leftover `{`, no `Unknown variable` errors, fr/es/pt/ar/hi pass a gender-agreement check against this file (a colour with a selector gets its feminine form exactly when the animal is `f`).
