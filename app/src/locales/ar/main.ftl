# PCTwin screens, Arabic (ar). Translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = صفحة PCTwin الرئيسية
language-label = اللغة

footer-free = النقل الشخصي مجاني. لا حاجة إلى حساب.
footer-help = مساعدة

## F01 Welcome

welcome-eyebrow = فصلك التالي جاهز.
welcome-title-line1 = إعدادك الكامل.
welcome-title-line2 = حاسوبك المحمول التالي.
welcome-body = ملفاتك وتطبيقاتك وإعداداتك المألوفة تنتقل معك.
welcome-start = بدء نقل كامل

## F06 Old or new laptop

role-back = رجوع
role-eyebrow = بدء نقل كامل
role-title = أي حاسوب محمول هذا؟
role-body = افتح PCTwin على الحاسوبين المحمولين، ثم حدّد أيهما هذا.
role-old-title = حاسوبي المحمول القديم
role-old-body = إرسال ملفاتي وتطبيقاتي وإعداداتي.
role-new-title = حاسوبي المحمول الجديد
role-new-body = استلام إعدادي الكامل من حاسوب محمول آخر.
role-note = لا يُنقل أي شيء قبل أن تراجع الخطة وتوافق عليها. يبقى حاسوبك المحمول القديم كما هو أثناء النقل.

## Frame parts (footer note, Back, steps)

footer-note-move = نقل شخصي مجاني
footer-note-send = إرسال قريب مجاني
frame-back = رجوع
# Read out by screen readers only; the step shapes carry the meaning on screen.
frame-step-done = تم
frame-step-current = الآن
frame-step-todo = التالي
frame-step-position = الخطوة { $number } من { $total }

## F01 Unfinished move

resume-open = استئناف عملية النقل

## C01 How will you connect?

connect-eyebrow = عملية النقل
connect-title = كيف سيتصل الحاسوبان؟
connect-body = مهما كانت الطريقة التي تختارها، فالخطوات نفسها.
connect-wifi-title = Wi-Fi
connect-wifi-body = الحاسوبان المحمولان على شبكة Wi-Fi نفسها.
connect-wifi-status = متاح
connect-later = الاتصال بكابل شبكة أو بكابل الجسر من PCTwin، والنقل عبر محرك أقراص خارجي، كلها مخطط لها في إصدار لاحق.
connect-usb-note = لا يستطيع كابل USB العادي توصيل حاسوبين محمولين لإجراء النقل.
connect-use-wifi = استخدام Wi-Fi
# Added: sizes and speeds as measured, shown the same way on every connection screen.
connect-unit-mbps = ⁦{ NUMBER($value, maximumFractionDigits: 0) } MB/s⁩
connect-unit-gb = ⁦{ NUMBER($value, maximumFractionDigits: 1) } GB⁩

## C02 Put both laptops together

connect-prep-title = ضع الحاسوبين المحمولين بالقرب من بعضهما.
connect-prep-body = وصّلهما بشبكة Wi-Fi نفسها، ثم افتح PCTwin على كل منهما.
connect-prep-step-1-title = الانضمام إلى شبكة Wi-Fi نفسها
connect-prep-step-1-body = نسخ الملفات لا يحتاج إلى الإنترنت.
# $network is the Wi-Fi name this laptop is on.
connect-prep-step-1-network = الشبكة: { $network }.
connect-prep-step-2-title = السماح بالعثور على الحواسيب القريبة
connect-prep-step-2-body = إذا طلب Windows أو macOS الإذن، فاختر «السماح».
connect-prep-step-3-title = اختيار دور على كل حاسوب
connect-prep-step-3-body = أحدهما يرسل، والآخر يستلم.
connect-prep-note = قد تمنع شبكات Wi-Fi العامة أو شبكات الضيوف الحواسيب المحمولة من رؤية بعضها.
connect-prep-continue = المتابعة إلى الربط

## C06 Meet your other laptop

peer-eyebrow = عملية النقل
peer-title = تعرّف على حاسوبك المحمول الآخر.
peer-body = الحاسوبان المحمولان متصلان وجاهزان.
peer-sending = إرسال
peer-receiving = استلام
peer-old = الحاسوب المحمول القديم
peer-new = الحاسوب المحمول الجديد
peer-version-label = PCTwin
# $version is the PCTwin version on that laptop, for example "1.0.3".
peer-version = { $version }
peer-status-update = يحتاج إلى تحديث
peer-status-unsupported = في الانتظار
peer-ready-note = لا يعرض PCTwin ملفاتك إلا بعد ربط الحاسوبين المحمولين. إذا اختلف الإصدار، فحدّث PCTwin على كليهما.
peer-version-differs = إصدار PCTwin مختلف على هذين الحاسوبين المحمولين. حدّث PCTwin على كليهما للمتابعة.
peer-version-old-body = إصدار مختلف: { $version }.
peer-version-new-body = يجب أن يكون الإصدار نفسه على الحاسوبين المحمولين.
peer-update-title = تحديث PCTwin
peer-update-body = ثبّت الإصدار نفسه على الحاسوبين المحمولين.
# $from and $to are system names, for example "Windows 11" and "macOS". The mockup sentence does not use them.
peer-direction-unsupported = لا يستطيع هذا الإصدار من PCTwin النقل بين هذين النظامين بعد.
peer-continue = التحقق من الاتصال

## C07 Let’s check the connection

check-eyebrow = عملية النقل
check-title = لنتحقق من الاتصال.
check-body = يتحقق PCTwin من السرعة والمساحة والطاقة ليخطط للنقل ويقدّر الوقت.
check-measuring = جارٍ قياس الاتصال…
# The value next to it is already formatted, for example "62 MB/s".
check-speed-label = سرعة الاتصال
check-space-label = متاحة على الحاسوب الجديد
check-link-weak = الاتصال ضعيف، لذا قد يكون النقل بطيئًا. قد يساعد تقريب الحاسوبين المحمولين من موجّه Wi-Fi.
check-power-title = الطاقة
check-power-both = الحاسوبان المحمولان موصولان بالكهرباء.
check-power-battery = يعمل { $label } على البطارية. وصّله بالكهرباء حتى لا ينقطع النقل الطويل.
check-space-title = التخزين
check-space-body = المساحة المتاحة على الحاسوب المحمول الجديد: { $free }. قبل نقل أي شيء، سترى هل تتسع لاختياراتك.
check-internet-title = الإنترنت
check-internet-on = متصل. نسخ الملفات لا يحتاج إلى الإنترنت.
check-internet-off = غير متصل. لا بأس، فنسخ الملفات لا يحتاج إلى الإنترنت.
check-status-ready = جاهز
check-status-plug-in = وصّله بالكهرباء
check-status-connected = متاح
check-status-not-needed = غير مطلوب
# Added: what is said when something could not be read, so it is never shown as fine.
check-status-unknown = غير معروف
check-speed-unknown = تعذّر على PCTwin قياس الاتصال هذه المرة. ستنتقل الملفات مع ذلك؛ ينقص فقط تقدير الوقت.
check-space-unknown = تعذّر على PCTwin قراءة المساحة المتاحة على الحاسوب المحمول الجديد.
check-power-unknown = تعذّر على PCTwin معرفة هل { $label } موصول بالكهرباء.
check-internet-unknown = تعذّر على PCTwin معرفة هل هذا الحاسوب المحمول متصل بالإنترنت. لا بأس، فنسخ الملفات لا يحتاج إلى الإنترنت.
check-again = التحقق مرة أخرى
check-estimate-note = الوقت المقدّر ليس وعدًا ثابتًا.
check-continue = اختيار ما يُنقل

## C09 Waiting for your other laptop

reconnect-title = في انتظار حاسوبك المحمول الآخر.
reconnect-body = كل ما نُسخ وتم التحقق منه حتى الآن محفوظ.
# Added: the title of the saved-progress card (the mockup shows Move paused by connection).
reconnect-card-title = النقل متوقف مؤقتًا بسبب الاتصال
reconnect-progress =
    .aria-label = النقل متوقف مؤقتًا بسبب الاتصال
# $done is an already formatted size, for example "81.6 GB". $total is passed but not shown.
reconnect-saved = تم نسخ { $done } والتحقق منها حتى الآن. والباقي قادم.
# $ago is already formatted, for example "45 seconds ago".
reconnect-last-seen = آخر ظهور: { $ago }.
reconnect-status-offline = غير متصل
reconnect-status-trying = جارٍ إعادة الاتصال
reconnect-status-pending = في الانتظار
reconnect-trying = يحاول PCTwin إعادة الاتصال تلقائيًا… أبقِ الحاسوبين المحمولين مشغّلين وخارج وضع السكون وعلى شبكة Wi-Fi نفسها.
reconnect-expired = مضى وقت طويل لا يسمح بإعادة الاتصال تلقائيًا. اربط الحاسوبين المحمولين مرة أخرى، وسيتحقق PCTwin مما نُسخ بالفعل.
reconnect-unavailable = تعذّر الوصول إلى { $label }. تأكد أنه مشغّل وخارج وضع السكون، وأن PCTwin مفتوح عليه، وأنه على شبكة Wi-Fi نفسها.
reconnect-try-now = إعادة محاولة الاتصال
reconnect-pair-again = الربط مرة أخرى
reconnect-nothing-new = لم يصل أي شيء جديد منذ مدة. يواصل PCTwin المحاولة تلقائيًا، أو يمكنك إيقاف النقل. كل ما نُسخ وتم التحقق منه حتى الآن يبقى محفوظًا.
reconnect-details = إعادة الاتصال بأمان
reconnect-details-body = لا يمكن أن يعيد الاتصال إلا هذان الحاسوبان المحمولان، وفي كل مرة يستخدمان اتصالًا خاصًا جديدًا.

## C10 The connection has ended (dialog)

ended-title = انتهى الاتصال.
ended-body = عملية النقل محفوظة. الرمز القديم لم يعد يعمل.
ended-kept-title = محفوظ
ended-kept-body = سجل عملية النقل وكل ما نُسخ وتم التحقق منه.
ended-not-kept-title = غير محفوظ
ended-not-kept-body = الإذن بالمتابعة دون الربط مرة أخرى.
ended-pair-again = الربط مرة أخرى
ended-pair-again-body = أعد الاتصال، وسيتحقق PCTwin مما نُسخ بالفعل.
ended-see-moved = فتح عملية النقل المحفوظة
ended-see-moved-body = اطّلع على ما نُسخ وتم التحقق منه حتى الآن.
ended-home = العودة إلى الرئيسية

## Pairing, shared

pair-back = رجوع
pair-try-again = إعادة المحاولة
pair-more-details = مزيد من التفاصيل
pair-heads-up-title = قد يطرح عليك حاسوبك سؤالًا.
pair-heads-up-local-network = سيسألك جهاز Mac هل يمكن لـ PCTwin العثور على الأجهزة في شبكتك المحلية. اختر «السماح» حتى يتمكن الحاسوبان من العثور على بعضهما.
pair-heads-up-firewall = قد يسألك Windows هل يمكن لـ PCTwin قبول الاتصالات. اختر «السماح» حتى يتمكن حاسوبك الجديد من الوصول إلى هذا الحاسوب.
pair-heads-up-continue = متابعة

## C03 Old laptop: show the code

pair-old-eyebrow = الحاسوب المحمول القديم
pair-old-title = لنوصل الحاسوبين المحمولين.
pair-old-body = على حاسوبك المحمول الجديد، أدخل الرمز أدناه.
pair-old-next = بعد ذلك، ستطابق رقمًا يظهر على الحاسوبين المحمولين.
pair-old-network = أبقِ الحاسوبين المحمولين على شبكة Wi-Fi نفسها.
pair-old-waiting = في انتظار حاسوبك المحمول الجديد…
pair-old-starting = جارٍ تجهيز رمز…
pair-old-label-title = اسم هذا الحاسوب المحمول مؤقتًا
pair-old-label-hint = سيجد حاسوبك المحمول الجديد هذا الحاسوب بهذا الاسم.
pair-old-shuffle = اختيار اسم آخر
pair-old-rename = كتابة اسم
pair-old-rename-label = اسم هذا الحاسوب المحمول
pair-old-rename-note = يمكن لأي شخص على شبكة Wi-Fi هذه رؤية هذا الاسم.
pair-old-rename-save = استخدام هذا الاسم
pair-old-rename-cancel = إلغاء
pair-old-code-meta =
    { $seconds ->
        [zero] يتغير كل دقيقة. رمز جديد بعد { $seconds } ثانية.
        [one] يتغير كل دقيقة. رمز جديد بعد ثانية واحدة.
        [two] يتغير كل دقيقة. رمز جديد بعد ثانيتين.
        [few] يتغير كل دقيقة. رمز جديد بعد { $seconds } ثوانٍ.
        [many] يتغير كل دقيقة. رمز جديد بعد { $seconds } ثانية.
       *[other] يتغير كل دقيقة. رمز جديد بعد { $seconds } ثانية.
    }
pair-old-steps-title = على حاسوبك المحمول الجديد
pair-old-step-1 = افتح PCTwin واختر «حاسوبي المحمول الجديد».
pair-old-step-2 = إذا ظهرت قائمة، فاختر اسم هذا الحاسوب المحمول.
pair-old-step-3 = اكتب الرمز المكوّن من ستة أرقام الظاهر هنا.
pair-old-wait-title = رمز جديد في الطريق.
pair-old-wait-body =
    { $seconds ->
        [zero] لم تنجح هذه المحاولة. يظهر رمز جديد بعد { $seconds } ثانية.
        [one] لم تنجح هذه المحاولة. يظهر رمز جديد بعد ثانية واحدة.
        [two] لم تنجح هذه المحاولة. يظهر رمز جديد بعد ثانيتين.
        [few] لم تنجح هذه المحاولة. يظهر رمز جديد بعد { $seconds } ثوانٍ.
        [many] لم تنجح هذه المحاولة. يظهر رمز جديد بعد { $seconds } ثانية.
       *[other] لم تنجح هذه المحاولة. يظهر رمز جديد بعد { $seconds } ثانية.
    }
pair-old-locked-title = حاول شخص ما على هذه الشبكة إدخال رموز خاطئة.
pair-old-locked-body = بعد خمسة رموز خاطئة، توقف الربط لحماية هذا الحاسوب المحمول. لم تتم مشاركة أي شيء. إذا كنت أنت من حاول، فاختر «البدء من جديد» للحصول على رمز جديد.
pair-old-locked-hotspot = إذا كنت لا تعرف من حاول، فوصّل الحاسوبين المحمولين بنقطة الاتصال في هاتفك بدلًا من ذلك، ثم اختر «البدء من جديد».
pair-old-start-again = البدء من جديد
pair-old-timed-out = لم يُختر أي رقم في الوقت المحدد، لذا هذا رمز جديد.
pair-old-declined = لم تتطابق الأرقام، لذا لم تتم مشاركة أي شيء. هذا رمز جديد.
pair-old-left = أوقف الحاسوب المحمول الجديد الربط قبل اختيار رقم، لذا لم تتم مشاركة أي شيء. هذا رمز جديد.
pair-network-title = اختر شبكة Wi-Fi.
pair-network-hint = اختر الشبكة التي يتصل بها حاسوبك المحمول الجديد.

## C05 Old laptop: match the number

pair-choose-eyebrow = تأكيد الاتصال
pair-choose-title = اختر الرقم المطابق.
pair-choose-body = انظر إلى حاسوبك المحمول الجديد، واختر الرقم الذي يعرضه.
pair-choose-number = الرقم { $number }
pair-choose-confirm = تأكيد هذين الحاسوبين
pair-choose-renewed = ظهرت أرقام جديدة. اختر الرقم الظاهر على حاسوبك المحمول الجديد.
pair-choose-none = لا يطابق أي منها
pair-choose-note = إذا لم يعرض الحاسوب المحمول الجديد أي رقم، أو لم يطابقه أي من هذه الأرقام، فاختر «لا يطابق أي منها».
pair-choose-safe = لم تتم مشاركة أي شيء بعد.
pair-choose-checking = جارٍ التحقق…

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = الحاسوب المحمول الجديد
pair-new-searching-title = جارٍ البحث عن حاسوبك المحمول القديم…
pair-new-searching-body = أبقِ PCTwin مفتوحًا على حاسوبك المحمول القديم، وعلى شبكة Wi-Fi نفسها التي يتصل بها هذا الحاسوب.
pair-new-search-failed-title = تعذّر البحث عن حاسوبك القديم.
pair-new-list-title = أيها حاسوبك المحمول القديم؟
pair-new-list-body = اختر الاسم الذي يظهر على شاشة حاسوبك المحمول القديم.
pair-new-same-label = يحمل حاسوب محمول آخر قريب هذا الاسم. تحقق من شاشة الحاسوب القديم.
pair-new-needs-update = يعمل على هذا الحاسوب إصدار أقدم من PCTwin. حدِّث PCTwin عليه، ثم ابحث مرة أخرى.
pair-new-none-title = لا يوجد حاسوب محمول قديم.
pair-new-none-body = تأكد أن PCTwin مفتوح على حاسوبك المحمول القديم ويعرض رمزًا، وأن الحاسوبين على شبكة Wi-Fi نفسها. ثم ابحث مرة أخرى.
pair-new-search-again = البحث مرة أخرى
pair-old-same-role = يوجد حاسوب آخر قريب مضبوط أيضًا على أنه الحاسوب القديم. يجب أن يكون أحدهما الحاسوب الجديد.
pair-old-switch = هذا هو الحاسوب الجديد
pair-new-none-same-role = هل اختار الحاسوب الآخر أيضًا الحاسوب الجديد؟ إذن اختر الحاسوب القديم عليه، أو بدّل هذا الحاسوب.
pair-new-switch = هذا هو الحاسوب القديم
pair-new-code-title = أدخل الرمز.
pair-new-code-body = اكتب الرمز الظاهر على { $label }.
pair-new-code-label = رمز من ستة أرقام
pair-new-code-hint = اقرأه من شاشة حاسوبك المحمول القديم. يتغير كل دقيقة.
pair-new-connect = اتصال
pair-new-connecting = جارٍ الاتصال…
pair-new-other-laptop = اختيار حاسوب محمول آخر
pair-new-number-eyebrow = تأكيد الاتصال
pair-new-number-title = اختر هذا الرقم على حاسوبك المحمول القديم.
pair-new-number-tile = رقمك
pair-new-number-label = رقمك هو { $number }
pair-new-number-body = يعرض حاسوبك المحمول القديم ثلاثة أرقام. اختر هذا الرقم هناك.
pair-new-number-waiting = في انتظار حاسوبك المحمول القديم…

## Paired (both laptops)

pair-done-eyebrow = كل شيء جاهز
pair-done-title = تم ربط الحاسوبين المحمولين.
pair-done-body = حاسوباك المحمولان متصلان. بعد ذلك، يبحث حاسوبك القديم عن ملفاتك، وتوافق أنت على ما يُنقل على الحاسوب الجديد. لا يُنقل شيء قبل موافقتك.
pair-done-continue = متابعة

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = يجب أن يتكون الرمز من ستة أرقام. تحقق من شاشة الحاسوب المحمول القديم واكتبه مرة أخرى.
pairing-code-wrong = الرمز غير مطابق. تحقق من شاشة الحاسوب المحمول القديم وحاول مرة أخرى.
pairing-code-expired = تغيّر الرمز. اكتب الرمز الجديد من شاشة الحاسوب المحمول القديم.
pairing-busy = الحاسوب المحمول القديم مشغول بالربط مع حاسوب آخر. انتظر لحظة، ثم حاول مرة أخرى.
pairing-wait = يجهّز الحاسوب المحمول القديم رمزًا جديدًا. انتظر حتى يظهر، ثم اكتب الرمز الجديد.
pairing-locked = توقف الربط بعد محاولات خاطئة كثيرة. على الحاسوب المحمول القديم، اختر «البدء من جديد».
pairing-unreachable = تعذّر على هذا الحاسوب المحمول الوصول إلى الحاسوب القديم. تأكد أن الحاسوبين على شبكة Wi-Fi نفسها، ثم حاول مرة أخرى.
pairing-wrong-number = هذا الرقم لا يطابق رقم الحاسوب المحمول الجديد. لم تتم مشاركة أي شيء. اكتب الرمز الجديد على الحاسوب الجديد لتحاول مرة أخرى.
pairing-no-match = لم تتطابق الأرقام على الحاسوب المحمول القديم. لم تتم مشاركة أي شيء. اكتب الرمز الجديد من شاشة الحاسوب القديم.
pairing-timed-out = لم يُختر أي رقم في الوقت المحدد. حاول مرة أخرى بالرمز الجديد على الحاسوب المحمول القديم.
pairing-connection-lost = انقطع الاتصال بين الحاسوبين المحمولين. لم تتم مشاركة أي شيء. حاول مرة أخرى.
pairing-connection-lost-new = انقطع الاتصال بالحاسوب المحمول القديم. لم تتم مشاركة أي شيء.
pairing-different-version = إصدار PCTwin مختلف على الحاسوبين المحمولين. حدّث PCTwin على كليهما، ثم حاول مرة أخرى.
pairing-name-invalid = لا يمكن استخدام هذا الاسم. جرّب اسمًا أقصر فيه حروف وأرقام.
pairing-no-network = هذا الحاسوب المحمول غير متصل بشبكة Wi-Fi أو بشبكة سلكية. وصّله بشبكة، ثم حاول مرة أخرى.
pairing-choose-network = هذا الحاسوب المحمول متصل بأكثر من شبكة. اختر الشبكة التي يتصل بها حاسوبك الجديد:
pairing-local-network-refused = ربما يمنع جهاز Mac تطبيق PCTwin من العثور على الحواسيب المحمولة في هذه الشبكة. إذا سألك جهاز Mac للتو، فاختر «السماح» ثم حاول مرة أخرى. وإلا فافتح «إعدادات النظام»، واختر «الخصوصية والأمن» ثم «الشبكة المحلية»، وفعّل PCTwin، ثم حاول مرة أخرى.
pairing-local-network-refused-details = قد تمنع أيضًا شبكة VPN أو جدار حماية أو تطبيق تصفية الحواسيب من العثور على بعضها. إذا كان أحدها يعمل، فأوقفه أثناء الربط، ثم حاول مرة أخرى.
pairing-failed = لم ينجح الربط هذه المرة. لم تتم مشاركة أي شيء. حاول مرة أخرى.
pairing-failed-new = لم ينجح الاقتران هذه المرة. لم تتم مشاركة أي شيء. اكتب الرمز الجديد من شاشة الحاسوب القديم.

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = عملية النقل
people-title = طابِق كل شخص مع حسابه.
people-body = اختر أين تذهب ملفات كل شخص على الحاسوب المحمول الجديد.
people-col-old = الحاسوب المحمول القديم
people-col-new = الحاسوب المحمول الجديد
people-col-status = الحالة
people-you = أنت
# $name is the person's name on the old laptop. $account is an account name on the new laptop.
people-select =
    .aria-label = حساب { $name } على الحاسوب المحمول الجديد
people-option-existing = { $account } / الحاسوب المحمول الجديد
people-option-new = إنشاء حساب جديد
people-option-leave-out = استبعاد
people-option-choose = اختيار…
people-new-account-hint = يطلب الحاسوب المحمول الجديد الإذن قبل أن ينشئ حسابًا.
people-suggested = مقترح لأن الأسماء متطابقة. تأكد أنه الشخص نفسه.
# $account is an account name on the new laptop.
people-suggested-use = استخدام { $account }
people-status-ready = مؤكد
people-status-waiting = بانتظار موافقة صاحب الحساب
people-status-left-out = مستبعد
people-status-choose = يحتاج إلى اختيارك
people-status-changed = عدّلته أنت
people-waiting-body = يجب على { $name } تسجيل الدخول على الحاسوب المحمول القديم والموافقة قبل نقل الملفات. حتى ذلك الحين، تبقى الملفات دون أي تغيير.
people-just-me = لم يُعثر على الحاسوب المحمول القديم إلا على حسابك أنت.
people-changed-note = غيّرت المكان الذي تذهب إليه ملفات أحد الأشخاص. راجعه قبل المتابعة.
people-shared-title = العناصر المشتركة
people-shared-body = المجلدات المشتركة والمستندات العامة التي تخص كل من يستخدم الحاسوب المحمول القديم.
people-shared-select =
    .aria-label = أين تذهب العناصر المشتركة
people-shared-option-shared = المجلدات المشتركة على الحاسوب المحمول الجديد
people-shared-option-person = مع ملفات { $name }
people-shared-option-leave-out = استبعاد
people-note = لا يغيّر PCTwin كلمة مرور أي شخص أبدًا، ولا يتجاوز تسجيل دخول شخص آخر.
people-list-label = الأشخاص على الحاسوب المحمول القديم
people-todo =
    { $count ->
        [0] لكل من يمكن نقل ملفاته مكان يذهب إليه.
        [zero] لكل من يمكن نقل ملفاته مكان يذهب إليه.
        [one] لا يزال شخص واحد يحتاج إلى اختيار.
        [two] لا يزال شخصان يحتاجان إلى اختيار.
        [few] لا يزال { $count } أشخاص يحتاجون إلى اختيار.
        [many] لا يزال { $count } شخصًا يحتاجون إلى اختيار.
       *[other] لا يزال { $count } شخص يحتاجون إلى اختيار.
    }
people-waiting-count =
    { $count ->
        [zero] ينتظر { $count } شخص الموافقة. يمكنك المتابعة، وتبقى ملفاتهم دون تغيير حتى تتم الموافقة.
        [one] ينتظر شخص واحد الموافقة. يمكنك المتابعة، وتبقى الملفات دون تغيير حتى تتم الموافقة.
        [two] ينتظر شخصان الموافقة. يمكنك المتابعة، وتبقى ملفاتهما دون تغيير حتى تتم الموافقة.
        [few] ينتظر { $count } أشخاص الموافقة. يمكنك المتابعة، وتبقى ملفاتهم دون تغيير حتى تتم الموافقة.
        [many] ينتظر { $count } شخصًا الموافقة. يمكنك المتابعة، وتبقى ملفاتهم دون تغيير حتى تتم الموافقة.
       *[other] ينتظر { $count } شخص الموافقة. يمكنك المتابعة، وتبقى ملفاتهم دون تغيير حتى تتم الموافقة.
    }
people-continue = متابعة

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = عملية النقل
before-title = قبل أن تترك هذا الحاسوب المحمول.
before-body = تحقق من هذه الأمور ما دام حاسوبك المحمول القديم معك.
before-licences-title = تراخيص التطبيقات
before-licences-body = بعض التطبيقات تتطلب منك نقل ترخيصها بنفسك.
before-passwords-title = استعادة الحسابات
before-passwords-body = تأكد أنه لا يزال بإمكانك الدخول إلى حساباتك وإلى أي محركات أقراص مقفلة.
before-codes-title = أبقِ رموز تسجيل الدخول في متناولك
before-codes-body = إذا كان أحد حساباتك يرسل إليك رمزًا أو يستخدم تطبيقًا للتأكد من هويتك، فأبقِ ذلك الهاتف أو التطبيق قريبًا منك.
before-online-title = عمل غير محفوظ
before-online-body = احفظ أي مسودات، وتأكد أن الملفات المخزنة على الإنترنت محدّثة.
before-done =
    .aria-label = أنجزتُ هذا: { $item }
before-note = لا تكتب كلمات المرور أو مفاتيح الاسترداد هنا. إنهاء هذه القائمة لا يعني أن مسح الحاسوب المحمول القديم آمن.
before-empty = لا يلزم فعل أي شيء على حاسوبك المحمول القديم قبل النقل.
before-continue = العودة إلى خطة النقل

## S11 Get help with this task (a short problem code)

help-eyebrow = المساعدة والخصوصية
help-title = احصل على مساعدة في هذه المهمة.
help-body = شارك رمزًا قصيرًا يوضح لشخص آخر ما الذي حدث. راجع ما يتضمنه أولًا.
help-making = جارٍ إنشاء رمز…
help-code-title = رمز المشكلة
# $code is the code itself, for example "PCT-7F3K-21". Keep it unchanged.
help-code =
    .aria-label = رمز المشكلة { $code }
help-contains-title = ما يتضمنه
help-contains-problem = نوع المشكلة والمدى الذي وصلت إليه المهمة.
help-contains-step = خطوة النقل التي حدثت فيها المشكلة.
help-contains-versions = إصدار PCTwin، والنظام الذي يعمل عليه هذا الحاسوب، مثل Windows أو macOS.
help-never-title = ما لا يتضمنه أبدًا
help-chip-included = مضمّن
help-chip-excluded = غير مضمّن أبدًا
help-never-names = اسمك أو اسم أي شخص آخر.
help-never-files = محتوى ملفاتك، أو أي كلمات مرور.
help-never-network = أسماء الملفات أو المجلدات، أو مكان حفظ ملفاتك، أو أسماء شبكات Wi-Fi.
help-note = لا يمنح الرمز أي شخص إمكانية الوصول إلى الحاسوبين المحمولين.
help-copy = نسخ الرمز
help-copied = تم نسخ الرمز.
help-share = مشاركة
help-share-hint = أرسله عبر WhatsApp أو أي تطبيق.
help-share-text = رمز مشكلة PCTwin الخاص بي هو { $code }.
help-failed = تعذّر إنشاء رمز المشكلة. حاول مرة أخرى. إذا استمر الأمر، فصِف المشكلة بكلماتك.
help-try-again = إعادة المحاولة
help-copy-failed = تعذّر نسخ الرمز. حدّده في الأعلى وانسخه بنفسك.
help-home-eyebrow = مساعدة
help-home-title = مساعدة في المهمة التي أمامك.
help-home-body = أدلة قصيرة لأكثر ما يعيق عادةً. تعمل دون إنترنت.
help-home-more = مزيد من المساعدة على getpctwin.com
help-guide-find-title = لا يجد الحاسوبان بعضهما
help-guide-find-body = صِل الحاسوبين بشبكة Wi-Fi نفسها، وليس بشبكة الضيوف. أوقف أي VPN أثناء الاقتران. إن لم ينجح ذلك، شغّل نقطة الاتصال في هاتفك وصِل الحاسوبين بها.
help-guide-mac-network-title = يسأل جهاز Mac عن الشبكة المحلية
help-guide-mac-network-body = اختر السماح حتى يتمكن PCTwin من إيجاد حاسوبك الآخر. إذا اخترت عدم السماح، افتح إعدادات النظام ثم الخصوصية والأمن ثم الشبكة المحلية، وشغّل PCTwin.
help-guide-firewall-title = يسأل Windows عن جدار الحماية
help-guide-firewall-body = اسمح لـ PCTwin على الشبكات الخاصة، مثل Wi-Fi المنزل. يتواصل PCTwin مع حاسوبك الآخر فقط، على شبكتك الخاصة.
help-guide-stopped-title = توقف النقل في منتصفه
help-guide-stopped-body = لا يضيع شيء مما نُسخ وتم التحقق منه، ولم يتغير حاسوبك القديم. اقرن الحاسوبين من جديد: تُكتشف الملفات الموجودة على الحاسوب الجديد ولا تُنسخ مرتين.
help-guide-others-title = نقل أشياء أشخاص آخرين
help-guide-others-body = اخترهم في «من سيُنقل؟» على الحاسوب القديم. يوافق المسؤول مرة واحدة على كل حاسوب. لن ترى ملفاتهم أبدًا. تظهر كلمة المرور المؤقتة للحساب الجديد مرة واحدة في نافذة الحاسوب الخاصة، ويختارون كلمتهم عند أول تسجيل دخول.
help-guide-code-title = طلب المساعدة من شخص ما
help-guide-code-body = في أي صفحة مشكلة، اختر «الحصول على مساعدة». ينشئ PCTwin رمزًا قصيرًا يوضح ما حدث، دون أسماء أو ملفات أو كلمات مرور، لترسله عبر WhatsApp أو أي تطبيق.

## Label words (the name each laptop shows while pairing)

label-colour-red =
    { $gender ->
        [feminine] حمراء
       *[masculine] أحمر
    }
label-colour-orange =
    { $gender ->
        [feminine] برتقالية
       *[masculine] برتقالي
    }
label-colour-yellow =
    { $gender ->
        [feminine] صفراء
       *[masculine] أصفر
    }
label-colour-green =
    { $gender ->
        [feminine] خضراء
       *[masculine] أخضر
    }
label-colour-blue =
    { $gender ->
        [feminine] زرقاء
       *[masculine] أزرق
    }
label-colour-purple =
    { $gender ->
        [feminine] بنفسجية
       *[masculine] بنفسجي
    }
label-colour-pink =
    { $gender ->
        [feminine] وردية
       *[masculine] وردي
    }
label-colour-brown =
    { $gender ->
        [feminine] بنية
       *[masculine] بني
    }
label-colour-black =
    { $gender ->
        [feminine] سوداء
       *[masculine] أسود
    }
label-colour-white =
    { $gender ->
        [feminine] بيضاء
       *[masculine] أبيض
    }
label-colour-grey =
    { $gender ->
        [feminine] رمادية
       *[masculine] رمادي
    }
label-colour-gold =
    { $gender ->
        [feminine] ذهبية
       *[masculine] ذهبي
    }

label-animal-ant = نملة
label-animal-badger = غرير
label-animal-bat = خفاش
label-animal-bear = دب
label-animal-bee = نحلة
label-animal-beetle = خنفساء
label-animal-bison = بيسون
label-animal-butterfly = فراشة
label-animal-camel = جمل
label-animal-cat = قطة
label-animal-cheetah = شيتا
label-animal-chicken = دجاجة
label-animal-crab = سلطعون
label-animal-crane = كركي
label-animal-crocodile = تمساح
label-animal-crow = غراب
label-animal-deer = أيل
label-animal-dolphin = دلفين
label-animal-donkey = حمار
label-animal-dove = حمامة
label-animal-dragonfly = يعسوب
label-animal-duck = بطة
label-animal-eagle = نسر
label-animal-elephant = فيل
label-animal-falcon = صقر
label-animal-flamingo = فلامنغو
label-animal-fox = ثعلب
label-animal-frog = ضفدع
label-animal-gazelle = غزال
label-animal-swordfish = أبو سيف
label-animal-giraffe = زرافة
label-animal-goat = عنزة
label-animal-goose = إوزة
label-animal-gorilla = غوريلا
label-animal-hamster = همستر
label-animal-hawk = باز
label-animal-hedgehog = قنفذ
label-animal-armadillo = أرماديلو
label-animal-hippo = فرس النهر
label-animal-horse = حصان
label-animal-hummingbird = طائر طنان
label-animal-jaguar = يغوار
label-animal-jellyfish = قنديل البحر
label-animal-kangaroo = كنغر
label-animal-kingfisher = رفراف
label-animal-koala = كوالا
label-animal-ladybird = دعسوقة
label-animal-lemur = ليمور
label-animal-leopard = فهد
label-animal-lion = أسد
label-animal-lizard = سحلية
label-animal-llama = لاما
label-animal-lobster = كركند
label-animal-lynx = وشق
label-animal-macaw = ماكاو
label-animal-meerkat = ميركات
label-animal-mole = خلد
label-animal-moose = موظ
label-animal-pufferfish = سمكة منتفخة
label-animal-mouse = فأر
label-animal-octopus = أخطبوط
label-animal-ostrich = نعامة
label-animal-otter = قضاعة
label-animal-owl = بومة
label-animal-panda = باندا
label-animal-kiwi = كيوي
label-animal-parrot = ببغاء
label-animal-peacock = طاووس
label-animal-pelican = بجع
label-animal-penguin = بطريق
label-animal-puffin = ببغاء البحر
label-animal-rabbit = أرنب
label-animal-raccoon = راكون
label-animal-rhino = وحيد القرن
label-animal-robin = أبو الحناء
label-animal-salmon = سلمون
label-animal-seahorse = حصان البحر
label-animal-seal = فقمة
label-animal-shark = قرش
label-animal-sheep = خروف
label-animal-sloth = كسلان
label-animal-snail = حلزون
label-animal-sparrow = عصفور
label-animal-squirrel = سنجاب
label-animal-starfish = نجمة البحر
label-animal-stork = لقلق
label-animal-swan = بجعة
label-animal-tiger = نمر
label-animal-snake = ثعبان
label-animal-toucan = توكان
label-animal-turtle = سلحفاة
label-animal-walrus = حيوان الفظ
label-animal-whale = حوت
label-animal-wolf = ذئب
label-animal-woodpecker = نقار الخشب
label-animal-yak = ياك
label-animal-zebra = حمار وحشي
label-animal-dog = كلب
label-animal-cow = بقرة
label-animal-beaver = قندس

label-gender-ant = feminine
label-gender-badger = masculine
label-gender-bat = masculine
label-gender-bear = masculine
label-gender-bee = feminine
label-gender-beetle = feminine
label-gender-bison = masculine
label-gender-butterfly = feminine
label-gender-camel = masculine
label-gender-cat = feminine
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
label-gender-dragonfly = masculine
label-gender-duck = feminine
label-gender-eagle = masculine
label-gender-elephant = masculine
label-gender-falcon = masculine
label-gender-flamingo = masculine
label-gender-fox = masculine
label-gender-frog = masculine
label-gender-gazelle = masculine
label-gender-swordfish = masculine
label-gender-giraffe = feminine
label-gender-goat = feminine
label-gender-goose = feminine
label-gender-gorilla = masculine
label-gender-hamster = masculine
label-gender-hawk = masculine
label-gender-hedgehog = masculine
label-gender-armadillo = masculine
label-gender-hippo = masculine
label-gender-horse = masculine
label-gender-hummingbird = masculine
label-gender-jaguar = masculine
label-gender-jellyfish = masculine
label-gender-kangaroo = masculine
label-gender-kingfisher = masculine
label-gender-koala = masculine
label-gender-ladybird = feminine
label-gender-lemur = masculine
label-gender-leopard = masculine
label-gender-lion = masculine
label-gender-lizard = feminine
label-gender-llama = masculine
label-gender-lobster = masculine
label-gender-lynx = masculine
label-gender-macaw = masculine
label-gender-meerkat = masculine
label-gender-mole = masculine
label-gender-moose = masculine
label-gender-pufferfish = feminine
label-gender-mouse = masculine
label-gender-octopus = masculine
label-gender-ostrich = feminine
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
label-gender-sheep = masculine
label-gender-sloth = masculine
label-gender-snail = masculine
label-gender-sparrow = masculine
label-gender-squirrel = masculine
label-gender-starfish = feminine
label-gender-stork = masculine
label-gender-swan = feminine
label-gender-tiger = masculine
label-gender-snake = masculine
label-gender-toucan = masculine
label-gender-turtle = feminine
label-gender-walrus = masculine
label-gender-whale = masculine
label-gender-wolf = masculine
label-gender-woodpecker = masculine
label-gender-yak = masculine
label-gender-zebra = masculine
label-gender-dog = masculine
label-gender-cow = feminine
label-gender-beaver = masculine

# Translators: the laptop's name is a colour plus an animal, for example "Blue Fox".
# Arabic: animal first, then the colour, which agrees with the animal's gender.
label-picked = { $animal } { $colour }
label-unnamed = حاسوب محمول بلا اسم

## M02 Finding your files

scan-eyebrow = عملية النقل
scan-title = جارٍ البحث عن إعدادك المألوف.
scan-body = يُحفظ ما يجده PCTwin، فيمكن أن يستكمل البحث إذا توقف.
scan-progress =
    .aria-label = جارٍ البحث في الأماكن التي اخترتها
# $size is passed but not shown in the mockup line.
scan-found =
    { NUMBER($count) ->
        [zero] عُثر على { $count } ملف حتى الآن.
        [one] عُثر على ملف واحد حتى الآن.
        [two] عُثر على ملفين حتى الآن.
        [few] عُثر على { $count } ملفات حتى الآن.
        [many] عُثر على { $count } ملفًا حتى الآن.
       *[other] عُثر على { $count } ملف حتى الآن.
    }
scan-step-usual-title = الملفات والمجلدات
scan-step-usual-body = تم البحث في الأماكن التي يستطيع PCTwin قراءتها.
scan-step-other-title = مجلدات ومحركات أقراص أخرى
scan-step-other-body = مجلدات ومحركات أقراص تستخدمها خارج الأماكن المعتادة.
scan-step-protected-title = أماكن محمية
scan-step-protected-body =
    { NUMBER($count) ->
        [zero] لا يزال { $count } مكان بحاجة إلى إذنك.
        [one] لا يزال مكان واحد بحاجة إلى إذنك.
        [two] لا يزال مكانان بحاجة إلى إذنك.
        [few] لا تزال { $count } أماكن بحاجة إلى إذنك.
        [many] لا يزال { $count } مكانًا بحاجة إلى إذنك.
       *[other] لا يزال { $count } مكان بحاجة إلى إذنك.
    }
scan-step-done = تم
scan-step-current = الآن
scan-step-todo = التالي
scan-unreadable-note = الأماكن التي تعذّر على PCTwin قراءتها تبقى في القائمة، ولا تُحسب ضمن ما تم البحث فيه.
scan-status-not-searched = لم يتم البحث فيه
scan-paused = البحث متوقف مؤقتًا. كل ما عُثر عليه حتى الآن محفوظ.
scan-resumed = استُؤنف البحث من حيث توقف.
scan-partial =
    { NUMBER($count) ->
        [zero] لم يتم البحث بعد في { $count } مكان محمي.
        [one] لم يتم البحث بعد في مكان محمي واحد.
        [two] لم يتم البحث بعد في مكانين محميين.
        [few] لم يتم البحث بعد في { $count } أماكن محمية.
        [many] لم يتم البحث بعد في { $count } مكانًا محميًا.
       *[other] لم يتم البحث بعد في { $count } مكان محمي.
    }
scan-allow = السماح بالوصول
scan-pause = إيقاف مؤقت
scan-resume = متابعة البحث
scan-continue = عرض ما عُثر عليه

## M03 Everything you chose, in view

overview-eyebrow = عملية النقل
overview-title = كل ما اخترته، أمامك.
overview-body = راجع ما عثر عليه PCTwin قبل أن يُعدّ الخطة.
# The metric value next to each label is passed in already formatted.
overview-size-label = الملفات المحددة
overview-items-label =
    { NUMBER($count) ->
        [zero] ملف
        [one] ملف
        [two] ملف
        [few] ملفات
        [many] ملفًا
       *[other] ملف
    }
overview-attention-label =
    { NUMBER($count) ->
        [zero] بحاجة إلى انتباهك
        [one] بحاجة إلى انتباهك
        [two] بحاجة إلى انتباهك
        [few] بحاجة إلى انتباهك
        [many] بحاجة إلى انتباهك
       *[other] بحاجة إلى انتباهك
    }
overview-files-title = الملفات والمجلدات
overview-files-body = المستندات والصور والمجلدات الأخرى التي اخترتها.
overview-dest-title = أين تذهب
overview-dest-body = المكان المخصص لكل مجلد على الحاسوب المحمول الجديد.
overview-people-title = الأشخاص
overview-people-body = اختر الحساب على الحاسوب المحمول الجديد لكل شخص.
overview-review = مراجعة
overview-partial =
    { NUMBER($count) ->
        [zero] لم يتم البحث بعد في { $count } مكان محمي.
        [one] لم يتم البحث بعد في مكان محمي واحد.
        [two] لم يتم البحث بعد في مكانين محميين.
        [few] لم يتم البحث بعد في { $count } أماكن محمية.
        [many] لم يتم البحث بعد في { $count } مكانًا محميًا.
       *[other] لم يتم البحث بعد في { $count } مكان محمي.
    }
overview-allow = السماح بالوصول
overview-continue = متابعة

## M04 Keep what matters to you

files-eyebrow = عملية النقل
files-title = احتفظ بما يهمك.
files-body = اختر المجلدات ومحركات الأقراص الأخرى وأي أماكن أخرى.
files-search-label = البحث عن مجلد
files-search-placeholder = البحث في المجلدات
files-select =
    .aria-label = نقل { $name }
files-folder-detail =
    { NUMBER($count) ->
        [zero] { $size } · { $count } ملف
        [one] { $size } · ملف واحد
        [two] { $size } · ملفان
        [few] { $size } · { $count } ملفات
        [many] { $size } · { $count } ملفًا
       *[other] { $size } · { $count } ملف
    }
files-status-protected = يحتاج إلى إذن
files-protected-body = يحتاج PCTwin إلى إذنك لقراءة هذا المجلد. لا تُحسب ملفاته حتى تسمح بذلك.
files-allow = السماح بالوصول
files-status-added = أضفته أنت
files-add = إضافة مجلد آخر
files-remove =
    .aria-label = إزالة { $name }
files-no-limit = النقل المجاني بلا حد لحجم الملفات أو لعدد المجلدات التي تنقلها.
files-total =
    { NUMBER($count) ->
        [zero] { $count } مجلد مختار، والإجمالي { $size }
        [one] مجلد واحد مختار، والإجمالي { $size }
        [two] مجلدان مختاران، والإجمالي { $size }
        [few] { $count } مجلدات مختارة، والإجمالي { $size }
        [many] { $count } مجلدًا مختارًا، والإجمالي { $size }
       *[other] { $count } مجلد مختار، والإجمالي { $size }
    }
files-none = اختر مجلدًا واحدًا على الأقل للمتابعة.
files-continue = اختيار الوجهات

## M05 Give every file a home

dest-eyebrow = عملية النقل
dest-title = امنح كل ملف مكانًا.
dest-body = اختر أين يذهب كل مجلد على الحاسوب المحمول الجديد.
dest-col-chosen = ما اخترته
dest-col-place = أين يذهب
dest-col-placement = محرك الأقراص
dest-placement-local = داخلي
dest-placement-external = خارجي
dest-col-status = الحالة
dest-select =
    .aria-label = وجهة { $name }
# $folder is a folder name on the new laptop, for example "Documents".
dest-option-folder = الحاسوب المحمول الجديد / { $folder }
# $drive is the name of a drive connected to the new laptop.
dest-option-drive = محرك أقراص خارجي / { $drive }
dest-option-choose = اختيار مجلد
dest-default-label = المكان المعتاد
dest-status-ready = جاهز
dest-status-missing = محرك الأقراص غير متصل
dest-status-space = المساحة غير كافية
dest-all-ready = لكل مجلد مكان على الحاسوب المحمول الجديد.
dest-missing = محرك الأقراص { $drive } غير موصول بالحاسوب المحمول الجديد. وصّله، أو اختر مكانًا آخر لهذه الملفات.
dest-space = في { $place } مساحة متاحة قدرها { $free }، لكن هذه الملفات تحتاج إلى { $needed }. وفّر مساحة هناك، أو اختر مكانًا آخر، أو انقل ملفات أقل.
dest-change = تغيير
dest-continue = مراجعة الخطة

## M23 Approve the move you want

plan-eyebrow = عملية النقل
plan-title = وافق على عملية النقل التي تريدها.
plan-body = لا يفعل PCTwin إلا ما تنص عليه هذه الخطة.
plan-what-title = ما اخترته
# $size is already formatted, for example "120 GB". $count is passed but not shown.
plan-what-body = ملفات بحجم { $size }، من المجلدات التي اخترتها، إلى الأماكن التي حددتها.
plan-where-body =
    { NUMBER($count) ->
        [zero] إلى { $count } مكان على الحاسوب المحمول الجديد.
        [one] إلى مكان واحد على الحاسوب المحمول الجديد.
        [two] إلى مكانين على الحاسوب المحمول الجديد.
        [few] إلى { $count } أماكن على الحاسوب المحمول الجديد.
        [many] إلى { $count } مكانًا على الحاسوب المحمول الجديد.
       *[other] إلى { $count } مكان على الحاسوب المحمول الجديد.
    }
plan-hand-title = قبل أن تبدأ
plan-hand-body =
    { NUMBER($count) ->
        [zero] { $count } خطوة. ستجدها في تقريرك في النهاية.
        [one] خطوة واحدة. ستجدها في تقريرك في النهاية.
        [two] خطوتان. ستجدهما في تقريرك في النهاية.
        [few] { $count } خطوات. ستجدها في تقريرك في النهاية.
        [many] { $count } خطوة. ستجدها في تقريرك في النهاية.
       *[other] { $count } خطوة. ستجدها في تقريرك في النهاية.
    }
plan-hand-none = لا شيء، بحسب ما يستطيع PCTwin معرفته الآن.
plan-old-safe = يبقى حاسوبك المحمول القديم كما هو. تُنسخ الملفات ولا تُزال منه أبدًا.
plan-edit = تغيير الخطة
plan-edit-note = إذا غيّرت الخطة، فستوافق عليها مرة أخرى قبل أن يبدأ أي شيء جديد.
plan-approve = أوافق على هذه الخطة، بما في ذلك ما يجب أن أفعله بنفسي.
plan-not-approved = لم توافق على هذه الخطة بعد، لذا لن يُنقل أي شيء.
plan-changed = غيّرت الخطة، لذا وافق عليها مرة أخرى قبل نقل أي شيء جديد.
plan-start = بدء النقل

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = رجوع
# $n is a count of files or folders.
plan-number = { NUMBER($n) }
# $n is a number already rounded for reading, for example 18.4.
plan-size-b = { NUMBER($n) } بايت
plan-size-kb = ⁦{ NUMBER($n) } KB⁩
plan-size-mb = ⁦{ NUMBER($n) } MB⁩
plan-size-gb = ⁦{ NUMBER($n) } GB⁩
plan-size-tb = ⁦{ NUMBER($n) } TB⁩
# $size is a size already written out, for example "18.4 GB". Used when part of a folder could not be read.
plan-size-at-least = { $size } على الأقل
plan-size-unknown = الحجم غير معروف
plan-role-home = المجلد الرئيسي
plan-role-desktop = سطح المكتب
plan-role-documents = المستندات
plan-role-downloads = التنزيلات
plan-role-pictures = الصور
plan-role-music = الموسيقى
plan-role-videos = مقاطع الفيديو
plan-role-public = المجلد العام
plan-role-app-data = بيانات التطبيقات
plan-role-other = مجلد آخر
# $done and $total are counts of places this search has looked at and has to look at.
scan-places = الأماكن التي تم البحث فيها: { $done } من { $total }
scan-card-title = جارٍ البحث في الأماكن التي اخترتها
scan-pausing = سيتوقف البحث مؤقتًا بعد المجلد الحالي. كل ما عُثر عليه حتى الآن محفوظ.
overview-row-attention = يحتاج إلى انتباه
files-status-work = العمل أو الدراسة
files-work-body = تدير جهة عملك أو دراستك هذا المجلد، لذا يُستبعد ما لم تحدده.
# $count is how many files in the folder are only online.
files-online-only =
    { NUMBER($count) ->
        [zero] لن يُنسخ { $count } ملف لأنها موجودة على الإنترنت فقط.
        [one] لن يُنسخ ملف واحد لأنه موجود على الإنترنت فقط.
        [two] لن يُنسخ ملفان لأنهما موجودان على الإنترنت فقط.
        [few] لن تُنسخ { $count } ملفات لأنها موجودة على الإنترنت فقط.
        [many] لن يُنسخ { $count } ملفًا لأنها موجودة على الإنترنت فقط.
       *[other] لن يُنسخ { $count } ملف لأنها موجودة على الإنترنت فقط.
    }
files-no-match = لا يوجد مجلد بهذا الاسم.
dest-status-none = اختر مكانًا
dest-status-readonly = لا يمكن الكتابة عليه
dest-status-format = الملف كبير جدًا
# $name is the folder that needs a place.
dest-none = اختر مكانًا للمجلد { $name } على الحاسوب المحمول الجديد.
# $place is a drive on the new laptop that cannot be written to.
dest-readonly = المكان { $place } للقراءة فقط، لذا لا يمكن نسخ أي شيء إليه. اختر مكانًا آخر.
# $place is the drive. Some drive formats cannot hold a single very large file.
dest-format = أحد الملفات في هذه المجلدات كبير جدًا بالنسبة إلى طريقة إعداد { $place }. اختر مكانًا آخر.
# $folder is a folder on the new laptop and $drive the drive it is on.
plan-place = { $folder } على { $drive }
plan-group-to = يذهب إلى { $place }
plan-group-move = نقل { $folder }
plan-place-change = اختر مكانًا آخر
plan-place-usual = استخدم المكان المعتاد
plan-place-free = { $size } متاحة هناك
plan-place-no-room = لا توجد مساحة كافية هناك. اختر مكانًا آخر، أو ألغِ تحديد هذا المجلد.
plan-place-no-safe-name = لا يستطيع PCTwin حفظ الملفات بأمان على ذلك القرص. اختر مكانًا آخر.
plan-place-cannot-open = تعذّر على PCTwin فتح ذلك المجلد. اختر مكانًا آخر.
plan-version = إصدار الخطة { $revision }
plan-details = مزيد من التفاصيل
plan-preview-title = قبل أن تبدأ
plan-preview-fits = توجد مساحة فارغة كافية على هذا الحاسوب.
plan-preview-short = يحتاج هذا الحاسوب إلى { $size } إضافية من المساحة الفارغة. وفّر بعض المساحة قبل البدء، وإلا سيتوقف النقل عند امتلاء القرص.
plan-preview-space-unknown = تعذّر على PCTwin معرفة المساحة الفارغة على هذا الحاسوب.
plan-preview-time-one =
    { NUMBER($minutes) ->
        [zero] نحو { $minutes } دقيقة.
        [one] نحو دقيقة واحدة.
        [two] نحو دقيقتين.
        [few] نحو { $minutes } دقائق.
        [many] نحو { $minutes } دقيقة.
       *[other] نحو { $minutes } دقيقة.
    }
plan-preview-time = نحو { $low } إلى { $high } دقيقة.
plan-preview-renamed =
    { NUMBER($count) ->
        [zero] { $count } ملف سيحصل على اسم مختلف قليلًا لأن هذا الحاسوب لا يسمح باسمه. يذكره التقرير.
        [one] ملف واحد سيحصل على اسم مختلف قليلًا لأن هذا الحاسوب لا يسمح باسمه. يذكره التقرير.
        [two] ملفان سيحصلان على اسمين مختلفين قليلًا لأن هذا الحاسوب لا يسمح باسميهما. يذكرهما التقرير.
        [few] { $count } ملفات ستحصل على أسماء مختلفة قليلًا لأن هذا الحاسوب لا يسمح بأسمائها. يذكر التقرير كل ملف.
        [many] { $count } ملفًا سيحصل على اسم مختلف قليلًا لأن هذا الحاسوب لا يسمح بأسمائها. يذكر التقرير كل ملف.
       *[other] { $count } ملف سيحصل على اسم مختلف قليلًا لأن هذا الحاسوب لا يسمح بأسمائها. يذكر التقرير كل ملف.
    }
plan-preview-too-big =
    { NUMBER($count) ->
        [zero] { $count } ملف أكبر من أن يتسع له قرص هذا الحاسوب ولن يُنسخ.
        [one] ملف واحد أكبر من أن يتسع له قرص هذا الحاسوب ولن يُنسخ.
        [two] ملفان أكبر من أن يتسع لهما قرص هذا الحاسوب ولن يُنسخا.
        [few] { $count } ملفات أكبر من أن يتسع لها قرص هذا الحاسوب ولن تُنسخ.
        [many] { $count } ملفًا أكبر من أن يتسع لها قرص هذا الحاسوب ولن تُنسخ.
       *[other] { $count } ملف أكبر من أن يتسع لها قرص هذا الحاسوب ولن تُنسخ.
    }
plan-preview-refused =
    { NUMBER($count) ->
        [zero] { $count } ملف له اسم أو مجلد لا يسمح به هذا الحاسوب، ولن يُنسخ.
        [one] ملف واحد له اسم أو مجلد لا يسمح به هذا الحاسوب، ولن يُنسخ.
        [two] ملفان لهما اسمان أو مجلدان لا يسمح بهما هذا الحاسوب، ولن يُنسخا.
        [few] { $count } ملفات لها أسماء أو مجلدات لا يسمح بها هذا الحاسوب، ولن تُنسخ.
        [many] { $count } ملفًا لها أسماء أو مجلدات لا يسمح بها هذا الحاسوب، ولن تُنسخ.
       *[other] { $count } ملف لها أسماء أو مجلدات لا يسمح بها هذا الحاسوب، ولن تُنسخ.
    }
plan-preview-runs-at-sign-in =
    { NUMBER($count) ->
        [zero] { $count } ملف سيعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
        [one] ملف واحد سيعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
        [two] ملفان سيعملان تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
        [few] { $count } ملفات ستعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
        [many] { $count } ملفًا سيعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
       *[other] { $count } ملف سيعمل تلقائيًا عند تسجيل الدخول إلى هذا الحاسوب. تأكد من أنك تتوقع ذلك.
    }
# $reference is a short code that names exactly this version of the plan.
plan-reference = المرجع { $reference }
# $count is how many files are affected in each line below.
plan-hand-cloud-only =
    { NUMBER($count) ->
        [zero] هناك { $count } ملف على الإنترنت فقط، ولا ينزّلها PCTwin. اجعلها متاحة على الحاسوب المحمول القديم أولًا، أو اتركها.
        [one] هناك ملف واحد على الإنترنت فقط، ولا ينزّله PCTwin. اجعله متاحًا على الحاسوب المحمول القديم أولًا، أو اتركه.
        [two] هناك ملفان على الإنترنت فقط، ولا ينزّلهما PCTwin. اجعلهما متاحين على الحاسوب المحمول القديم أولًا، أو اتركهما.
        [few] هناك { $count } ملفات على الإنترنت فقط، ولا ينزّلها PCTwin. اجعلها متاحة على الحاسوب المحمول القديم أولًا، أو اتركها.
        [many] هناك { $count } ملفًا على الإنترنت فقط، ولا ينزّلها PCTwin. اجعلها متاحة على الحاسوب المحمول القديم أولًا، أو اتركها.
       *[other] هناك { $count } ملف على الإنترنت فقط، ولا ينزّلها PCTwin. اجعلها متاحة على الحاسوب المحمول القديم أولًا، أو اتركها.
    }
plan-hand-unreadable =
    { NUMBER($count) ->
        [zero] تعذّر فتح { $count } عنصر، لذا لن تُنقل. اسمح بالوصول، أو اتركها.
        [one] تعذّر فتح عنصر واحد، لذا لن يُنقل. اسمح بالوصول، أو اتركه.
        [two] تعذّر فتح عنصرين، لذا لن يُنقلا. اسمح بالوصول، أو اتركهما.
        [few] تعذّر فتح { $count } عناصر، لذا لن تُنقل. اسمح بالوصول، أو اتركها.
        [many] تعذّر فتح { $count } عنصرًا، لذا لن تُنقل. اسمح بالوصول، أو اتركها.
       *[other] تعذّر فتح { $count } عنصر، لذا لن تُنقل. اسمح بالوصول، أو اتركها.
    }
plan-hand-work-managed =
    { NUMBER($count) ->
        [zero] هناك { $count } عنصر تديرها جهة عملك أو دراستك، لذا استُبعدت. حدّد مجلداتها لتضمينها.
        [one] هناك عنصر واحد تديره جهة عملك أو دراستك، لذا استُبعد. حدّد مجلده لتضمينه.
        [two] هناك عنصران تديرهما جهة عملك أو دراستك، لذا استُبعدا. حدّد مجلديهما لتضمينهما.
        [few] هناك { $count } عناصر تديرها جهة عملك أو دراستك، لذا استُبعدت. حدّد مجلداتها لتضمينها.
        [many] هناك { $count } عنصرًا تديرها جهة عملك أو دراستك، لذا استُبعدت. حدّد مجلداتها لتضمينها.
       *[other] هناك { $count } عنصر تديرها جهة عملك أو دراستك، لذا استُبعدت. حدّد مجلداتها لتضمينها.
    }
plan-blocked-someone-unmapped = هناك شخص على الحاسوب المحمول القديم لم يُختر له حساب على الجديد بعد. اختر أين تذهب ملفات كل شخص، ثم عُد.
plan-blocked-account-used-twice = سينتهي شخصان في حساب واحد على الحاسوب المحمول الجديد. خصّص لكل شخص حسابه.
plan-blocked-invalid = لا يمكن الموافقة على هذه الخطة كما هي. ارجع وراجع ما اخترته.
plan-blocked-fix = اختيار الحسابات

## Tray menu

tray-show = إظهار PCTwin
tray-quit = إنهاء PCTwin

## F01 secondary actions and their coming-soon pages

welcome-restore = استعادة الإعداد الكامل
welcome-rescue = إنقاذ حاسوب محمول
welcome-quick-send = الإرسال السريع

soon-eyebrow = قريبًا
soon-restore-title = استعادة الإعداد الكامل
soon-restore-body = أعد بناء حاسوب محمول من نسخة الإعداد الكامل التي حفظتها بعد عملية نقل. هذه الميزة قادمة في إصدار لاحق.
soon-rescue-title = إنقاذ حاسوب محمول
soon-rescue-body = استرجع الملفات من حاسوب محمول لا يعمل، بتوصيل محرك أقراصه بحاسوب يعمل. هذه الميزة قادمة في إصدار لاحق.

## X01 Move progress, compact

progress-eyebrow = عملية النقل
progress-title = نهيّئ لك مكانك الجديد.
progress-title-paused = النقل متوقف مؤقتًا
progress-paired = تم الربط
progress-percent =
    .aria-label = تقدّم النقل
# $done and $total are already formatted sizes, for example "81.6 GB" and "120 GB".
progress-amount = { $done } من { $total }
# $range is a measured range, already formatted, for example "15 to 20 minutes".
progress-time-left = يتبقى نحو { $range }
progress-time-measuring = جارٍ حساب الوقت المتبقي…
# $speed is measured and already formatted, for example "62 MB/s".
progress-speed = { $speed }، حسب القياس
progress-count-needs-action =
    { NUMBER($count) ->
        [zero] { $count } عنصر بحاجة إلى إجراء منك
        [one] عنصر واحد بحاجة إلى إجراء منك
        [two] عنصران بحاجة إلى إجراء منك
        [few] { $count } عناصر بحاجة إلى إجراء منك
        [many] { $count } عنصرًا بحاجة إلى إجراء منك
       *[other] { $count } عنصر بحاجة إلى إجراء منك
    }
progress-current-copying = يُنسخ الآن: { $name }
progress-current-checking = يجري التحقق الآن من: { $name }
progress-needs-action =
    { NUMBER($count) ->
        [zero] { $count } عنصر بحاجة إلى إجراء منك. ويستمر نقل الباقي.
        [one] عنصر واحد بحاجة إلى إجراء منك. ويستمر نقل الباقي.
        [two] عنصران بحاجة إلى إجراء منك. ويستمر نقل الباقي.
        [few] { $count } عناصر بحاجة إلى إجراء منك. ويستمر نقل الباقي.
        [many] { $count } عنصرًا بحاجة إلى إجراء منك. ويستمر نقل الباقي.
       *[other] { $count } عنصر بحاجة إلى إجراء منك. ويستمر نقل الباقي.
    }
progress-see-needs-action = عرض ما يحتاج إليك
progress-interrupted = انقطع الاتصال بالحاسوب المحمول الآخر. يحاول PCTwin إعادة الاتصال. لم يضِع أي شيء نُسخ وتم التحقق منه.
progress-low-battery = بطارية هذا الحاسوب منخفضة. وصّله بالكهرباء حتى لا يتوقف النقل.
progress-battery-paused = بطارية هذا الحاسوب على وشك النفاد، لذلك توقف النقل مؤقتًا بأمان. وصّله بالكهرباء ثم تابع. لم يضِع أي شيء.
progress-old-safe = يبقى حاسوبك المحمول القديم كما هو.
progress-pause = إيقاف النقل مؤقتًا
progress-compact = عرض مختصر
progress-expand = عرض عملية النقل كاملة
progress-stop = إيقاف النقل

## X02 Move progress, expanded

progress-all-title = عملية النقل كاملة.
progress-all-body = يُظهر كل عنصر من أين أتى وكيف يسير.
progress-metric-overall = التقدم الإجمالي
progress-metric-committed = تم نسخه والتحقق منه
progress-metric-estimate = تقدير
progress-col-item = العنصر
progress-col-from = من
progress-col-status = الحالة
progress-state-waiting = في الانتظار
progress-state-copying = جارٍ النسخ
progress-state-verified = تم التحقق
progress-state-checking = جارٍ التحقق
progress-state-paused = متوقف مؤقتًا
progress-checked-note = لا يُعدّ العنصر منسوخًا إلا بعد التحقق منه.

## F09 Mini panel

mini-title = عملية النقل، بالقرب منك.
mini-title-paused = النقل متوقف مؤقتًا.
mini-what = نقل الملفات التي حددتها
mini-progress =
    .aria-label = تقدّم النقل
mini-pause = إيقاف مؤقت
mini-resume = استئناف
mini-open = فتح عملية النقل
# Used when lane E reuses this panel for a Quick Send.
mini-title-send = ملفاتك في الطريق.
mini-what-send = إرسال الملفات المحددة
mini-open-send = فتح الإرسال
mini-progress-send =
    .aria-label = تقدّم الإرسال

## X05 Paused at a safe point

pause-title = متوقف مؤقتًا عند نقطة آمنة.
pause-title-finishing = جارٍ الإيقاف المؤقت عند نقطة آمنة…
pause-body = كل ما نُسخ وتم التحقق منه محفوظ. والباقي ينتظر.
pause-finishing = جارٍ إنهاء { $name } أولًا، حتى لا يبقى منسوخًا جزئيًا.
pause-kept-title = الملفات
pause-kept-body = كل ما نُسخ وتم التحقق منه حتى الآن محفوظ.
pause-waiting-title = في الانتظار
pause-waiting-body =
    { NUMBER($count) ->
        [zero] لا يزال { $count } عنصر بانتظار النسخ. لم يضِع أي شيء.
        [one] لا يزال عنصر واحد بانتظار النسخ. لم يضِع أي شيء.
        [two] لا يزال عنصران بانتظار النسخ. لم يضِع أي شيء.
        [few] لا تزال { $count } عناصر بانتظار النسخ. لم يضِع أي شيء.
        [many] لا يزال { $count } عنصرًا بانتظار النسخ. لم يضِع أي شيء.
       *[other] لا يزال { $count } عنصر بانتظار النسخ. لم يضِع أي شيء.
    }
pause-status-kept = متوقف مؤقتًا
pause-status-waiting = في الانتظار
pause-note = الإيقاف المؤقت لا يلغي أي شيء. يبقى حاسوبك المحمول القديم كما هو.
pause-resume = استئناف النقل المحفوظ

## X10 Stop the remaining move? (danger dialog)

cancel-title = هل تريد إيقاف ما تبقى من النقل؟
cancel-body = الإيقاف يحتفظ بكل ما نُسخ وتم التحقق منه، ولا يلغي ما تم نقله.
cancel-copied-title = الملفات المنسوخة
cancel-copied-body = تبقى على الحاسوب المحمول الجديد.
cancel-rest-title = باقي عملية النقل
cancel-rest-body = لن يبدأ أي شيء جديد. الملفات التي لم تُنسخ بعد تبقى على الحاسوب المحمول القديم فقط.
cancel-partial-title = ملف نُسخ جزئيًا
cancel-partial-body = لا يُحتفظ به أبدًا كأنه مكتمل.
cancel-old-title = الحاسوب المحمول القديم
cancel-old-body = يبقى كما هو.
cancel-ack = أفهم أن الإيقاف لا يلغي ما تم نسخه
cancel-stop = الإيقاف وعرض التقرير
cancel-keep = متابعة النقل

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = نتائج النقل
outcome-title = مرحبًا بك في حاسوبك المحمول التالي.
outcome-body = هذا ما حدث لكل ما وافقت عليه.
outcome-files-title = الملفات المحددة
outcome-status-partial = تم جزئيًا
outcome-next-title = الخطوات التالية
outcome-old-safe = كل ما لم يكتمل يبقى مدرجًا هنا. لم يُحذف أي شيء من حاسوبك المحمول القديم.
outcome-report = عرض التقرير الكامل
outcome-home = العودة إلى الرئيسية
outcome-person-copied =
    { NUMBER($count) ->
        [zero] نُسخ { $count } ملف.
        [one] نُسخ ملف واحد.
        [two] نُسخ ملفان.
        [few] نُسخت { $count } ملفات.
        [many] نُسخ { $count } ملفًا.
       *[other] نُسخ { $count } ملف.
    }
outcome-person-not-done =
    { NUMBER($count) ->
        [zero] لم يكتمل { $count }.
        [one] لم يكتمل ملف واحد.
        [two] لم يكتمل ملفان.
        [few] لم تكتمل { $count } ملفات.
        [many] لم يكتمل { $count } ملفًا.
       *[other] لم يكتمل { $count } ملف.
    }
outcome-person-at-sign-in = ستكون في مكانها عندما يسجّل { $name } الدخول إلى هذا الحاسوب في المرة القادمة.
move-fallback-folder = من حاسوبك القديم
move-stop = إيقاف النقل
move-before-open = أثناء الانتظار: قبل أن تترك هذا الحاسوب
move-get-help = الحصول على مساعدة
pickup-eyebrow = عملية النقل
pickup-title = تابع بأمان.
pickup-body = توقفت عملية نقل على هذا الحاسوب في منتصفها، ربما لأنه أُعيد تشغيله أو انقطعت عنه الكهرباء. رتّب الأمور أولًا، ثم اقرن الحاسوبين من جديد للمتابعة.
pickup-unfinished-title = نسخ غير مكتملة
pickup-unfinished-body =
    { NUMBER($count) ->
        [zero] كان { $count } ملف قيد النسخ عندما توقف النقل.
        [one] كان ملف واحد قيد النسخ عندما توقف النقل.
        [two] كان ملفان قيد النسخ عندما توقف النقل.
        [few] كانت { $count } ملفات قيد النسخ عندما توقف النقل.
        [many] كان { $count } ملفًا قيد النسخ عندما توقف النقل.
       *[other] كان { $count } ملف قيد النسخ عندما توقف النقل.
    }
pickup-unfinished-badge = بحاجة إلى ترتيب
pickup-tidy = ترتيب
pickup-tidying = جارٍ الترتيب. يتم التحقق من النسخ المكتملة ووضعها في مكانها؛ ولا يتغير أي شيء يخصك.
pickup-done = تم الترتيب. اقرن الحاسوبين من جديد للمتابعة: ما هو موجود هنا بالفعل يُكتشف ولا يُنسخ مرتين.
pickup-failed = لم ينجح الترتيب هذه المرة. لم يتغير أي شيء يخصك. حاول مرة أخرى، أو أغلق PCTwin على هذا الحاسوب وافتحه من جديد أولًا.
pickup-put-in-place = وُضعت في مكانها
pickup-kept = محفوظة للمتابعة
pickup-not-finished =
    { NUMBER($count) ->
        [zero] تعذّر إكمال { $count } ملف. سيُنسخ من جديد في النقل التالي.
        [one] تعذّر إكمال ملف واحد. سيُنسخ من جديد في النقل التالي.
        [two] تعذّر إكمال ملفين. سيُنسخان من جديد في النقل التالي.
        [few] تعذّر إكمال { $count } ملفات. ستُنسخ من جديد في النقل التالي.
        [many] تعذّر إكمال { $count } ملفًا. ستُنسخ من جديد في النقل التالي.
       *[other] تعذّر إكمال { $count } ملف. ستُنسخ من جديد في النقل التالي.
    }
pickup-left-for-later =
    { NUMBER($count) ->
        [zero] { $count } ملف على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
        [one] ملف واحد على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
        [two] ملفان على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
        [few] { $count } ملفات على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
        [many] { $count } ملفًا على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
       *[other] { $count } ملف على قرص غير متصل الآن. وصّله ثم رتّب مرة أخرى.
    }
pickup-later = ليس الآن
ready-eyebrow = مرحبًا
ready-title = أشياؤك جاهزة.
ready-body = نُقلت أشياؤك من حاسوبك القديم إلى هذا الحاسوب، وهي الآن في مجلداتك الخاصة.
ready-landed = ملف في مجلداتك
ready-where-title = أين هي
ready-where-body = في المستندات والصور والموسيقى ومقاطع الفيديو ومجلداتك المعتادة الأخرى، كما كانت على حاسوبك القديم.
ready-kept =
    { NUMBER($count) ->
        [zero] احتُفظ بـ { $count } ملف جانبًا لأن ملفًا بالاسم نفسه كان موجودًا هنا. احتُفظ بالاثنين.
        [one] احتُفظ بملف واحد جانبًا لأن ملفًا بالاسم نفسه كان موجودًا هنا. احتُفظ بالاثنين.
        [two] احتُفظ بملفين جانبًا لأن ملفات بالأسماء نفسها كانت موجودة هنا. احتُفظ بالجميع.
        [few] احتُفظ بـ { $count } ملفات جانبًا لأن ملفات بالأسماء نفسها كانت موجودة هنا. احتُفظ بالجميع.
        [many] احتُفظ بـ { $count } ملفًا جانبًا لأن ملفات بالأسماء نفسها كانت موجودة هنا. احتُفظ بالجميع.
       *[other] احتُفظ بـ { $count } ملف جانبًا لأن ملفات بالأسماء نفسها كانت موجودة هنا. احتُفظ بالجميع.
    }
ready-failed =
    { NUMBER($count) ->
        [zero] تعذّر وضع { $count } ملف في مجلداتك. اطلب من الشخص الذي أجرى النقل نقله مرة أخرى.
        [one] تعذّر وضع ملف واحد في مجلداتك. اطلب من الشخص الذي أجرى النقل نقله مرة أخرى.
        [two] تعذّر وضع ملفين في مجلداتك. اطلب من الشخص الذي أجرى النقل نقلهما مرة أخرى.
        [few] تعذّر وضع { $count } ملفات في مجلداتك. اطلب من الشخص الذي أجرى النقل نقلها مرة أخرى.
        [many] تعذّر وضع { $count } ملفًا في مجلداتك. اطلب من الشخص الذي أجرى النقل نقلها مرة أخرى.
       *[other] تعذّر وضع { $count } ملف في مجلداتك. اطلب من الشخص الذي أجرى النقل نقلها مرة أخرى.
    }
ready-done = تم
move-status-kept = محفوظ
move-wait-eyebrow = عملية النقل
move-wait-scanning-title = جارٍ البحث عن ملفاتك.
move-wait-scanning-body = يتصفح حاسوبك القديم مجلداتك. إنه يقرأها فقط: لا يُفتح شيء ولا يُغيَّر ولا يُحذف.
move-found = ما وُجد حتى الآن: { $size } من أشيائك.
move-wait-approval-title = في انتظار حاسوبك الجديد.
move-wait-approval-body = ملفاتك جاهزة. راجع الخطة على حاسوبك الجديد ووافق عليها للبدء.
move-wait-list-title = في انتظار حاسوبك القديم.
move-wait-list-body = يبحث حاسوبك القديم عن ملفاتك. عندما ينتهي، تظهر الخطة هنا لتوافق عليها. قد يستغرق ذلك بضع دقائق.
move-wait-starting-title = جارٍ بدء النقل.
move-wait-starting-body = تمت الموافقة على الخطة. يبدأ النسخ بعد لحظات.
move-wait-admin-title = في انتظار موافقة المسؤول.
move-wait-admin-body = يطلب هذا الحاسوب موافقة المسؤول لنقل أشياء الأشخاص الآخرين أيضًا. أجب في النافذة التي فتحها حاسوبك.
move-others-not-approved = لم يوافق المسؤول، لذلك تُنقل أشياؤك فقط هذه المرة. تبقى أشياء الأشخاص الآخرين في مكانها.
move-others-failed = تعذّر على PCTwin تشغيل ما يحتاجه لنقل أشياء الأشخاص الآخرين، لذلك تُنقل أشياؤك فقط هذه المرة. لم يتغير أي شيء يخصهم.
move-others-not-here = نقل أشياء الأشخاص الآخرين من هذا الحاسوب غير متاح بعد في هذا الإصدار، لذلك تُنقل أشياؤك فقط.
move-others-allow-in-settings = يجب أولًا أن يسمح مسؤول لـ PCTwin في صفحة إعدادات النظام التي فُتحت الآن. تُنقل أشياؤك فقط هذه المرة؛ وبعد السماح لـ PCTwin، يمكن لنقلك التالي أن يأخذ أشياء الأشخاص الآخرين أيضًا.
approve-prompt = سيقرأ PCTwin ملفات { $names } لنقلها إلى الحاسوب الجديد. يوافق مسؤول هذا الـ Mac على هذا النقل فقط.
who-here-title = من سيُنقل؟
who-here-body = تُنقل أشياؤك دائمًا. اختر أي شخص آخر على هذا الحاسوب يجب نقل أشيائه أيضًا. لا يعرض لك PCTwin ملفاتهم أبدًا.
who-here-you = { $name } (أنت)
who-here-you-hint = يُنقل دائمًا
who-here-approval = نقل أشياء شخص آخر يحتاج إلى موافقة المسؤول مرة واحدة لكل من تختارهم. سيطلبها حاسوبك في نافذته الخاصة.
who-here-not-here = نقل أشياء الأشخاص الآخرين من هذا الحاسوب غير متاح بعد في هذا الإصدار. تُنقل أشياؤك فقط.
place-title = إلى أين تذهب أشياؤهم؟
place-body = جاءت أشياء أشخاص آخرين من الحاسوب القديم أيضًا. اختر لكل شخص حسابًا على هذا الحاسوب، أو حسابًا جديدًا له، أو استبعده. لن ترى ملفاتهم أبدًا.
place-size =
    { NUMBER($count) ->
        [zero] { $count } ملف، { $size }
        [one] ملف واحد، { $size }
        [two] ملفان، { $size }
        [few] { $count } ملفات، { $size }
        [many] { $count } ملفًا، { $size }
       *[other] { $count } ملف، { $size }
    }
place-select = مكان أشياء { $name }
place-note = تحتاج الحسابات الجديدة، وحفظ أشيائهم حتى يسجلوا الدخول، إلى موافقة المسؤول مرة واحدة بعد موافقتك على الخطة. تظهر كلمة المرور المؤقتة لكل حساب جديد مرة واحدة في نافذة حاسوبك الخاصة.
place-todo =
    { NUMBER($count) ->
        [zero] اختر مكان أشياء { $count } أشخاص آخرين.
        [one] اختر مكان أشياء شخص واحد آخر.
        [two] اختر مكان أشياء شخصين آخرين.
        [few] اختر مكان أشياء { $count } أشخاص آخرين.
        [many] اختر مكان أشياء { $count } شخصًا آخر.
       *[other] اختر مكان أشياء { $count } شخص آخر.
    }
fda-title = جهاز Mac يُبقي بعض المجلدات خاصة.
fda-body =
    { NUMBER($count) ->
        [zero] تعذّر البحث في { $count } من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. اسمح لـ PCTwin بالدخول ثم تحقق مرة أخرى حتى لا يفوتك شيء.
        [one] تعذّر البحث في مكان واحد من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. اسمح لـ PCTwin بالدخول ثم تحقق مرة أخرى حتى لا يفوتك شيء.
        [two] تعذّر البحث في مكانين من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. اسمح لـ PCTwin بالدخول ثم تحقق مرة أخرى حتى لا يفوتك شيء.
        [few] تعذّر البحث في { $count } أماكن من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. اسمح لـ PCTwin بالدخول ثم تحقق مرة أخرى حتى لا يفوتك شيء.
        [many] تعذّر البحث في { $count } مكانًا من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. اسمح لـ PCTwin بالدخول ثم تحقق مرة أخرى حتى لا يفوتك شيء.
       *[other] تعذّر البحث في { $count } مكان من أماكنك بسبب إعدادات الخصوصية في جهاز Mac. اسمح لـ PCTwin بالدخول ثم تحقق مرة أخرى حتى لا يفوتك شيء.
    }
fda-step-1 = افتح إعدادات النظام، ثم الخصوصية والأمن.
fda-step-2 = اختر الوصول الكامل إلى القرص.
fda-step-3 = شغّل PCTwin. قد يطلب جهاز Mac كلمة المرور.
fda-open = افتح هذه الإعدادات
fda-note = يقرأ PCTwin مجلداتك فقط لنسخها. ولا يغيّر أي شيء على هذا الحاسوب.
fda-again = تحقق مرة أخرى
fda-without = تابع بدونها
password-title = PCTwin: حساب جديد
password-body =
    أصبح لدى { $name } حساب على هذا الحاسوب.
    
    اسم تسجيل الدخول: { $account }
    كلمة المرور المؤقتة: { $password }
    
    أعطِ هذه المعلومات لـ { $name }. عند أول تسجيل دخول يختار كلمة مروره الخاصة. دوّنها الآن: لا يحتفظ بها PCTwin.
move-ended-eyebrow = عملية النقل
move-ended-kept-title = ما نُسخ بالفعل
move-ended-kept-body = كل ما نُسخ وتم التحقق منه قبل حدوث ذلك يبقى على الحاسوب الجديد.
move-ended-old-title = حاسوبك القديم
move-ended-old-body = لم يُغيَّر أو يُحذف أي شيء عليه.
move-ended-old-badge = دون تغيير
move-ended-next = للمحاولة مرة أخرى، اقرن الحاسوبين من جديد. تُكتشف الملفات المنسوخة والمتحقق منها ولا تُنسخ مرتين.
move-lost-title = انقطع الاتصال بحاسوبك الآخر.
move-lost-body = توقف الحاسوبان عن الوصول إلى بعضهما، فلم يكتمل النقل. تأكد من أن كليهما يعمل وغير نائم وقريب من الآخر.
move-not-approved-title = لم يبدأ النقل.
move-not-approved-old = لم تتم الموافقة على الخطة على الحاسوب الجديد، لذلك لم يُنسخ شيء.
move-not-approved-new = لم توافق على الخطة، لذلك لم يُنسخ شيء.
move-failed-title = حدث خطأ ما.
move-failed-body = تعذّر متابعة النقل. إذا تكرر ذلك، يوضح قسم مزيد من التفاصيل أدناه ما الذي حدث.
move-peer-system-windows = Windows
move-peer-system-mac = Mac
move-peer-system-linux = Linux
move-peer-system-other = نظام آخر
move-update-here-title = حدّث PCTwin على هذا الحاسوب
move-update-here-body = إصدار PCTwin على هذا الحاسوب أقدم من الحاسوب الآخر، لذا لا يمكنهما نقل الملفات معًا. حدّث PCTwin هنا ثم ابدأ من جديد. لم يُنسخ أي شيء.
move-update-there-title = حدّث PCTwin على الحاسوب الآخر
move-update-there-body = إصدار PCTwin على الحاسوب الآخر أقدم من هذا الحاسوب، لذا لا يمكنهما نقل الملفات معًا. حدّث PCTwin هناك ثم ابدأ من جديد. لم يُنسخ أي شيء.
move-refused-title = تعذّر بدء النقل.
move-refused-not-paired = لم يعد حاسوباك مقترنين. اقرنهما من جديد لبدء النقل.
move-refused-busy = هناك عملية نقل جارية بالفعل على هذا الحاسوب.
move-refused-not-moving = لا توجد عملية نقل جارية على هذا الحاسوب.
move-refused-plan-changed = تغيرت الخطة قبل الموافقة عليها. اقرن الحاسوبين من جديد لرؤية الخطة الجديدة.
move-refused-failed = تعذّر تجهيز مجلدات هذا الحاسوب للنقل. اقرن الحاسوبين من جديد للمحاولة مرة أخرى.
drive-problem-title = يحتاج النقل إلى انتباهك.
drive-problem-body = لم تتطابق النسخ المحفوظة في { $place } مع الأصل عندما تحقق منها PCTwin، وتكرر ذلك عدة مرات. قد يكون القرص معطلًا أو ممتلئًا أو مفصولًا، لذلك توقف النسخ إليه مؤقتًا.
drive-problem-kept-title = ما نُسخ وتم التحقق منه
drive-problem-waiting-title = ما زال يلزم نسخه إلى { $place }
drive-problem-badge = متوقف مؤقتًا
drive-problem-fix = تأكد من توصيل القرص ووجود مساحة فارغة عليه، ثم تابع. لا يُستبعد أي شيء اخترته لتجاوز ذلك. يمكنك أيضًا إيقاف النقل.
drive-problem-carry-on = متابعة

## R02 Your move, item by item (detailed report)

report-title = عملية النقل، عنصرًا عنصرًا.
report-body = ما حدث لكل عنصر، وسببه، ومن أين أتى وإلى أين ذهب.
report-filter-label = عرض
report-col-item = العنصر
report-col-result = النتيجة
report-col-details = التحقق أو الخطوة التالية
report-from = من: { $path }
report-to = إلى: { $path }
report-none = لا توجد عناصر بهذه النتيجة.
report-more-details = مزيد من التفاصيل

# Evidence: how a copied item was checked.
evidence-read-back = تم التحقق بقراءته مرة أخرى من محرك الأقراص.
evidence-memory = تم التحقق عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
evidence-already-there = موجود بالفعل على الحاسوب المحمول الجديد: كان الملف نفسه هناك، لذلك لم يُنسخ مرة أخرى.
evidence-recovered = تم التحقق منه قبل إعادة تشغيل الحاسوب المحمول، ثم مرة أخرى بعدها.
evidence-new-laptop = تحقق منه الحاسوب المحمول الجديد.
evidence-memory-explain = قُرئت النسخة مرة أخرى وتطابقت، لكن عبر ذاكرة هذا الحاسوب المحمول، لذا ربما جاء ما قُرئ من الذاكرة لا من محرك الأقراص. قد لا يظهر هنا عطل في محرك الأقراص نفسه، لذا من الجيد فتح الملف مرة واحدة للتأكد.
evidence-not-checked = لم يتم التحقق، لأنه لم يكتمل.
# Machine-translated 16 October 2026.
why-memory-cannot-skip-cache = لم يسمح محرك الأقراص هذا لـ PCTwin بقراءته مباشرة، لذا تم التحقق عبر ذاكرة هذا الحاسوب المحمول. تعمل بعض محركات أقراص الشبكة وذواكر USB بهذه الطريقة.
why-memory-not-shown = لم يتمكن محرك الأقراص هذا من إظهار أن قراءة PCTwin وصلت إليه فعلًا، لذا يُحسب التحقق كأنه تم عبر ذاكرة هذا الحاسوب المحمول.
why-memory-no-way = لا يملك نظام هذا الحاسوب المحمول طريقة لقراءة محرك الأقراص هذا مباشرة، لذا تم التحقق عبر ذاكرته.
why-memory-stored-specially = هذا الملف مخزّن بطريقة خاصة، مثل الضغط أو التشفير، لذا يقرؤه النظام عبر الذاكرة.
why-memory-too-small = هذا الملف صغير جدًا، لذا يحفظه محرك الأقراص مع سجلاته الخاصة ويقرؤه النظام عبر الذاكرة.

# Reasons: why an item did not finish. Each says what happened, then what to do.
reason-not-enough-space = نفدت المساحة على الحاسوب المحمول الجديد. وفّر مساحة عليه، ثم ابدأ عملية نقل جديدة تشمل هذا العنصر.
reason-changed-during-move = ظل هذا الملف يتغير على الحاسوب المحمول القديم أثناء نسخه. أغلق أي تطبيق يستخدمه، ثم ابدأ عملية نقل جديدة تشمله.
reason-unreadable = لم يسمح الحاسوب المحمول القديم بأن يقرأ PCTwin هذا العنصر. تأكد أنه يُفتح على الحاسوب القديم، ثم ابدأ عملية نقل جديدة تشمله.
reason-not-verified = النسخة لم تطابق الأصل، لذا لم يُحتفظ بها. ابدأ عملية نقل جديدة تشمل هذا العنصر. إذا فشلت مرة أخرى، فقد يكون الأصل تالفًا.
reason-refused-by-safety-check = رفض فحص الأمان على الحاسوب المحمول الجديد هذا العنصر بسبب اسمه أو مكانه أو حجمه. لا يزال الأصل على الحاسوب القديم، لذا يمكنك نسخه بطريقة أخرى إذا احتجت إليه.
reason-name-not-allowed = لا يمكن استخدام هذا الاسم على الحاسوب المحمول الجديد، ولم يكن هناك اسم جديد آمن ممكن. غيّر اسمه على الحاسوب القديم، ثم ابدأ عملية نقل جديدة تشمله.
reason-install-failed = لم يُثبَّت التطبيق. ثبّته من موقع الجهة المطوّرة له.
reason-no-equivalent = لا يوجد إصدار من هذا التطبيق لنظام الحاسوب المحمول الجديد. ابحث عن تطبيق مشابه يعمل عليه.
reason-sign-in = أصبح في مكانه، لكن عليك تسجيل الدخول. افتحه وسجّل الدخول.
reason-activation = أصبح التطبيق في مكانه، لكنه يحتاج إلى ترخيصه على الحاسوب المحمول الجديد. افتحه وفعّله بترخيصك أو حسابك.
reason-permission = يجب أن يمنح الحاسوب المحمول الجديد الإذن أولًا. افتح العنصر واختر «السماح» عند السؤال.
reason-online-only = هذا الملف مخزّن على الإنترنت فقط، لذا لم يكن على الحاسوب المحمول القديم لنسخه. نزّله على الحاسوب الجديد من التطبيق الذي يخزّنه على الإنترنت.
reason-manual-install = يجب الحصول على هذا التطبيق من موقع الجهة المطوّرة له. نزّله وثبّته من هناك.
reason-restart = يجب إعادة تشغيل الحاسوب المحمول الجديد لإكمال العملية. أعد تشغيله عندما يناسبك.
reason-finish-by-hand = لا يستطيع PCTwin تنفيذ هذه الخطوة نيابةً عنك. اتبع الخطوات المعروضة لها في هذا التقرير.

## Item results (every item ends with one of these)

status-copied = تم النسخ
status-reinstalled = أُعيد التثبيت
status-converted = تم التحويل
status-needs-action = يحتاج إلى إجراء
status-skipped-by-user = تخطّيته أنت
status-failed = فشل

## Screens: words the copy deck did not cover (same voice)

# $low and $high are whole minutes, a measured range. The result fills "About { $range } left".
progress-minutes = من { $low } إلى { $high } دقيقة
# Machine-translated 16 October 2026.
progress-minutes-one =
    { NUMBER($minutes) ->
        [zero] { $minutes } دقيقة
        [one] دقيقة واحدة
        [two] دقيقتين
        [few] { $minutes } دقائق
        [many] { $minutes } دقيقة
       *[other] { $minutes } دقيقة
    }
# $low and $high are numbers and $unit is already formatted, for example "MB/s". The result fills "{ $speed }, measured".
progress-speed-range = من { $low } إلى ⁦{ $high } { $unit }⁩
progress-count-failed =
    { NUMBER($count) ->
        [zero] تعذّر نسخ { $count } عنصر
        [one] تعذّر نسخ عنصر واحد
        [two] تعذّر نسخ عنصرين
        [few] تعذّر نسخ { $count } عناصر
        [many] تعذّر نسخ { $count } عنصرًا
       *[other] تعذّر نسخ { $count } عنصر
    }
progress-queue-empty = لم يبدأ أي شيء بعد.
progress-approved-job = عملية النقل الموافق عليها
progress-route-old = الحاسوب المحمول القديم
progress-route-new = الحاسوب المحمول الجديد
progress-panel-pausing = يتوقف مؤقتًا
outcome-metric-size = تم نسخه والتحقق منه
outcome-metric-action = بحاجة إلى إجراء منك
progress-percent-unknown = لا يزال PCTwin يحسب حجم عملية النقل هذه.
mini-title-send-paused = الإرسال متوقف مؤقتًا.
cancel-ack-hint = حدّد المربع أعلاه لتفعيل زر الإيقاف.
progress-state-placing = جارٍ وضعه في مكانه
progress-state-existing = موجود مسبقًا
outcome-stopped-title = تم إيقاف عملية النقل.
outcome-stopped-body = لقد أوقفت عملية النقل. ما نُسخ وتم التحقق منه قبل ذلك محفوظ. والباقي لم يُنسخ.
# $done and $total are counts of items: what was copied and checked, and everything the person approved.
outcome-files-summary = العناصر التي نُسخت وتم التحقق منها: { $done } من { $total }.
outcome-files-memory =
    { NUMBER($count) ->
        [zero] تم التحقق من { $count } عنصر عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
        [one] تم التحقق من عنصر واحد عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
        [two] تم التحقق من عنصرين عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
        [few] تم التحقق من { $count } عناصر عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
        [many] تم التحقق من { $count } عنصرًا عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
       *[other] تم التحقق من { $count } عنصر عبر ذاكرة هذا الحاسوب المحمول، لا عبر محرك الأقراص.
    }
report-show-more = عرض المزيد
# $shown and $total are counts of items.
report-showing = عرض { $shown } من { $total }

## Component kit

dialog-close = إغلاق
toast-dismiss = إخفاء
code-label = رمز من ستة أرقام

## Shared Quick Send words

qs-eyebrow = الإرسال السريع
qs-eyebrow-receiving = الاستلام على هذا الحاسوب المحمول
qs-label-this = هذا الحاسوب
qs-label-other = الحاسوب الآخر
qs-wait-title = في انتظار الملفات.
qs-wait-body = الحاسوبان مقترنان. سترى ما يُعرض قبل استلام أي شيء.
qs-pick-failed = تعذّر فتح نافذة الاختيار. حاول مرة أخرى.
qs-problem-lost-title = انقطع الاتصال بالحاسوب الآخر.
qs-problem-failed-title = تعذّر إرسال الملفات هذه المرة.
qs-problem-body = يبقى كل ما استُلم وتم التحقق منه في مكانه. اقرن الحاسوبين من جديد للمحاولة مرة أخرى.
# The route between the two laptops, for example "Blue Fox" to "Green Owl".
qs-route =
    .aria-label = من { $from } إلى { $to }
qs-route-sender = إرسال · هذا الحاسوب
qs-route-receiver = استلام · شبكة Wi-Fi نفسها
qs-tally =
    { NUMBER($count) ->
        [zero] { $count } عنصر محدد · { $size }
        [one] عنصر واحد محدد · { $size }
        [two] عنصران محددان · { $size }
        [few] { $count } عناصر محددة · { $size }
        [many] { $count } عنصرًا محددًا · { $size }
       *[other] { $count } عنصر محدد · { $size }
    }
qs-tally-badge = إرسال قريب

## Q01 Quick Send home

qs-home-title = بضعة ملفات، مباشرةً.
qs-home-body = أرسل بضعة ملفات إلى حاسوب محمول قريب.
qs-send-title = إرسال الملفات
qs-send-body = اختر ملفات أو مجلدات على هذا الحاسوب المحمول.
qs-receive-title = استلام الملفات
qs-receive-body = اسمح لحاسوب محمول قريب بإرسال ملفات إلى هذا الحاسوب.
qs-history-title = إيصالاتك
qs-history-body = اعرف أين ذهبت الملفات المستلمة وما لم يكتمل.
qs-home-action = اختيار الملفات
qs-receiving = جاهز للاستلام. ترى الحواسيب المحمولة القريبة هذا الحاسوب باسم { $label }.
qs-receive-stop = إيقاف الاستلام
qs-no-nearby = لم يُعثر على حواسيب محمولة قريبة. افتح PCTwin على الحاسوب الآخر وأبقِ الحاسوبين على شبكة Wi-Fi نفسها.

## Q02 Choose files and folders

qs-choose-title = ما الذي سترسله؟
qs-choose-body = اختر فقط الملفات والمجلدات التي تريد مشاركتها.
qs-drop =
    .aria-label = منطقة إفلات الملفات والمجلدات
qs-drop-title = أفلِت الملفات أو المجلدات هنا
qs-drop-body = أو أضفها عبر «إضافة ملفات» أو «إضافة مجلدات».
qs-drop-active = أفلِتها لإضافتها.
qs-add-files = إضافة ملفات
qs-add-folders = إضافة مجلدات
qs-remove =
    .aria-label = إزالة { $name }
# $kind is the file type, for example "PDF document" or "8 photos".
qs-file-detail = { $size } · { $kind }
qs-empty = لم يُختر أي شيء بعد. أضف ملفات أو مجلدات لإرسالها.
qs-online-only = بعض الملفات موجودة على الإنترنت فقط. اجعلها متاحة على هذا الحاسوب المحمول أولًا، ثم أرسلها.
qs-protected = لا يستطيع PCTwin قراءة { $name }. اسمح بالوصول، أو أزله من القائمة.
qs-allow = السماح بالوصول
qs-choose-continue = التحقق من هذه الملفات

## Q03 Check what you are sharing

qs-check-title = تحقق مما ستشاركه.
qs-check-body = راجع هذه الملفات قبل أن تغادر هذا الحاسوب المحمول.
qs-check-included = مضمّن
qs-check-shared-note = يرى الحاسوب المحمول الآخر أسماء هذه الملفات وأحجامها وتواريخها.
qs-check-changed = تغيّر { $name } منذ أن اخترته. تحقق منه مرة أخرى.
qs-check-ack = لقد تحققت من هذه الملفات
qs-check-ack-hint = أزل أي شيء خاص قبل الإرسال.
qs-check-continue = اختيار الحاسوب الآخر

## Q04 Who should receive it?

qs-who-title = من سيستلمها؟
qs-who-body = اختر الاسم الظاهر على الحاسوب المحمول الآخر.
qs-who-nearby = قريب · شبكة Wi-Fi نفسها.
qs-who-code = استخدام رمز ربط
qs-who-code-body = أدخل الرمز الظاهر على حاسوب المرسل.
qs-who-same-label = يحمل حاسوبان محمولان قريبان هذا الاسم. تحقق من الاسم على شاشة الحاسوب الآخر.
qs-who-busy = الحاسوب { $label } مشغول بإرسال آخر. انتظر لحظة، ثم حاول مرة أخرى.
qs-who-note = يقبل الحاسوب المحمول الآخر الملفات ويختار أين تذهب.
qs-who-action = عرض رمز الإرسال

## Q08 Incoming offer (receiving laptop)

qs-offer-title = هناك من يريد أن يرسل إليك ملفات.
qs-offer-body = تحقق مما يُرسل قبل أن تقبل.
qs-offer-note = لا يحفظ PCTwin إلا الملفات التي تقبلها، ولا يفتحها أو يشغّلها أبدًا.
qs-offer-accept = القبول واختيار مجلد
qs-offer-decline = رفض هذه الملفات
qs-offer-declined = لقد رفضت. لم يُحفظ أي شيء على هذا الحاسوب المحمول.
qs-offer-changed = غيّر المرسل الملفات. تحقق من القائمة الجديدة قبل أن تقبل.
qs-offer-gone = لم يعد { $label } متاحًا. لم يُحفظ أي شيء على هذا الحاسوب المحمول.

## Q09 Where should the files land?

qs-land-title = أين تُحفظ الملفات؟
qs-land-body = اختر المجلد على هذا الحاسوب المحمول.
qs-land-folder-label = مجلد الاستلام
qs-land-choose = اختيار مجلد آخر
qs-land-space-title = المساحة المتاحة
qs-land-space = المتاح { $free } · المطلوب { $needed }.
qs-land-status-fits = تتسع
qs-land-status-no-space = المساحة غير كافية
qs-land-no-space = في هذا المجلد مساحة متاحة قدرها { $free }، لكن هذه الملفات تحتاج إلى { $needed }. اختر مجلدًا آخر أو وفّر مساحة.
qs-land-unavailable = هذا المجلد غير متاح. وصّل محرك الأقراص الخاص به، أو اختر مجلدًا آخر.
qs-land-denied = لا يملك PCTwin إذنًا بالحفظ في هذا المجلد. اختر مجلدًا آخر.
qs-land-note = إذا كان هناك ملف بالاسم نفسه، فستختار ما يحدث قبل حفظ أي شيء.
qs-land-confirm = استخدام هذا المجلد لهذه الملفات
qs-land-confirm-hint = لا تُحفظ هنا إلا الملفات التي قبلتها.
qs-land-use = استخدام هذا المجلد

## Q10 Keep your existing work safe

qs-conflict-title = احمِ عملك الموجود.
qs-conflict-body = اختر ما يحدث عندما يكون هناك ملف بالاسم نفسه.
qs-conflict-col-file = العنصر
qs-conflict-col-existing = موجود مسبقًا
qs-conflict-col-choice = الاختيار
# $when is already formatted, for example "yesterday".
qs-conflict-existing = آخر تعديل: { $when }
qs-conflict-none = لا توجد نسخة سابقة
qs-conflict-select =
    .aria-label = ما يحدث للعنصر { $name }
qs-keep-both = الاحتفاظ بالاثنين
qs-skip = تخطي هذا العنصر
qs-add-new = إضافة الجديد
qs-keep-both-note = يُبقي خيار «الاحتفاظ بالاثنين» الملف الموجود كما هو، ويُحفظ الجديد باسم { $newName }.
qs-no-conflicts = لا يوجد أي من هذه الملفات في المجلد بعد. لن يُستبدل أي شيء.
qs-conflict-target-changed = إذا تغيّر الملف الموجود أو تعذّر فتحه، فسينتظر حتى تتحقق منه مرة أخرى.
qs-conflict-approve = أوافق على هذه الاختيارات
qs-conflict-continue = تأكيد هذه الاختيارات

## Q11 Ready to send these files?

qs-approve-title = هل نرسل هذه الملفات؟
qs-approve-body = تحقق مما سيُرسل بالضبط قبل أن يبدأ.
qs-approve-receiver-title = الحاسوب الآخر
qs-approve-receiver-waiting = في انتظار أن يقبل الحاسوب المحمول الآخر ويختار مجلدًا.
qs-approve-receiver-ready = قبِل الحاسوب المحمول الآخر، واختار مجلدًا وما يحدث للملفات الموجودة مسبقًا.
qs-approve-files-title = الملفات
qs-approve-files-same = لا تغيّر هذه الملفات حتى تُرسل.
qs-approve-files-changed = تغيّرت بعض الملفات منذ أن تحققت منها. تحقق منها مرة أخرى.
qs-approve-existing-title = الملفات الموجودة
qs-approve-existing-body = الاحتفاظ بالاثنين، كما اخترت.
qs-status-waiting = في الانتظار
qs-status-accepted = تمت الموافقة
qs-status-unchanged = مراجعة
qs-status-changed = تغيّر
qs-approve-ack = إرسال هذه الملفات
qs-approve-ack-hint = أوافق على هذه الملفات وعلى هذا الحاسوب المحمول.
qs-approve-start = بدء الإرسال
qs-approve-cleared = غيّرت الملفات، لذا يجب أن يقبلها { $label } مرة أخرى.
qs-approve-withdrawn = لم يعد { $label } يقبل هذا الإرسال. لم يُرسل أي شيء.

## Q12 Send progress

qs-sending-title = ملفاتك في الطريق.
qs-sending-body = تنتقل النسخ مباشرةً من حاسوب محمول إلى آخر.
qs-sending-progress =
    .aria-label = جارٍ إرسال الملفات المحددة
qs-sending-current = يُرسل الآن: { $name }
# $speed is measured and already formatted, for example "3.1 MB/s".
qs-sending-speed = { $speed }، حسب القياس
qs-step-copying = النسخ
qs-step-copying-body =
    { NUMBER($done) ->
        [zero] تم استلام { $done } عنصر محدد.
        [one] تم استلام عنصر واحد محدد.
        [two] تم استلام عنصرين محددين.
        [few] تم استلام { $done } عناصر محددة.
        [many] تم استلام { $done } عنصرًا محددًا.
       *[other] تم استلام { $done } عنصر محدد.
    }
qs-step-checking = التحقق
qs-step-checking-body = يُقارن كل ملف بالأصل.
qs-step-receipt = الإيصال
qs-step-receipt-body = في النهاية، اعرف أين ذهبت الملفات وما لم يكتمل.
qs-sending-paused = متوقف مؤقتًا. الملفات التي استُلمت وتم التحقق منها محفوظة.
qs-sending-disconnected = انقطع الاتصال مع { $label }. الملفات التي تم التحقق منها محفوظة. تجري إعادة المحاولة تلقائيًا…
qs-sending-needs-action =
    { NUMBER($count) ->
        [zero] { $count } عنصر بحاجة إلى إجراء منك.
        [one] عنصر واحد بحاجة إلى إجراء منك.
        [two] عنصران بحاجة إلى إجراء منك.
        [few] { $count } عناصر بحاجة إلى إجراء منك.
        [many] { $count } عنصرًا بحاجة إلى إجراء منك.
       *[other] { $count } عنصر بحاجة إلى إجراء منك.
    }
qs-pause = إيقاف الإرسال مؤقتًا
qs-resume = استئناف
qs-compact = عرض مختصر
qs-stop = إيقاف هذا الإرسال
qs-sending-complete-title = اكتمل الإرسال
qs-sending-complete-body = كل ملف وصل طابق أصله. اعرف أين حُفظت الملفات.
qs-sending-receipt = عرض الإيصال

## Q14 Handover receipt (both laptops)

qs-receipt-title = هذا هو إيصالك.
qs-receipt-body = اعرف بالضبط ما وصل وأين حُفظ.
qs-receipt-box-title = وصلت ملفاتك
qs-receipt-box-body = قورنت كل نسخة بالأصل وتطابقت.
qs-receipt-arrived =
    { NUMBER($count) ->
        [zero] تم استلام { $count } عنصر محدد
        [one] تم استلام عنصر واحد محدد
        [two] تم استلام عنصرين محددين
        [few] تم استلام { $count } عناصر محددة
        [many] تم استلام { $count } عنصرًا محددًا
       *[other] تم استلام { $count } عنصر محدد
    }
qs-receipt-size = تطابق { $size } مع الأصل
qs-receipt-saved-as = { $path }
qs-receipt-not-arrived = لم يصل كل ما في هذا المجلد، أو لم يتم التحقق منه كله.
qs-receipt-needs-action = { $path } موجود، لكنه لم يكتمل. ما يجب فعله موجود في مزيد من التفاصيل.
qs-receipt-status-verified = تم التحقق
qs-receipt-status-unfinished = لم يكتمل
qs-receipt-not-completed = لم يكتمل.
qs-receipt-stopped-row = استُلم وتم التحقق منه قبل الإيقاف.
qs-receipt-partial =
    { NUMBER($count) ->
        [zero] لا يزال { $count } عنصر بحاجة إلى انتباه.
        [one] لا يزال عنصر واحد بحاجة إلى انتباه.
        [two] لا يزال عنصران بحاجة إلى انتباه.
        [few] لا تزال { $count } عناصر بحاجة إلى انتباه.
        [many] لا يزال { $count } عنصرًا بحاجة إلى انتباه.
       *[other] لا يزال { $count } عنصر بحاجة إلى انتباه.
    }
qs-receipt-stopped = تم إيقاف هذا الإرسال. الملفات التي وصلت كاملة تبقى في المجلد الذي اخترته.
qs-receipt-not-opened = لم يفتح PCTwin أي ملف مستلم ولم يشغّله.
qs-receipt-ack = تأكيد استلام الإيصال
# $when is already formatted, for example "today at 14:05".
qs-receipt-acked = تم تأكيد استلام الإيصال { $when }.
qs-receipt-ack-note = تأكيد استلام هذا الإيصال لا يعني أن الملفات فُتحت أو استُخدمت.
qs-receipt-offline = الحاسوب { $label } غير متصل، لذا لم يطّلع على هذا الإيصال بعد. الإيصال محفوظ على هذا الحاسوب المحمول.
qs-receipt-missing = إرسال الأجزاء الناقصة
qs-receipt-missing-body = تحقق فقط من الملفات التي لم تصل.
qs-receipt-history = كل الإيصالات
qs-receipt-done = العودة إلى الإرسال السريع

## Q19 Finish the missing part

qs-missing-title = أكمل الجزء الناقص.
qs-missing-body = استخدم الإيصال السابق لمعرفة ما لم يصل.
qs-missing-status-there = وصل
qs-missing-status-missing = يحتاج إلى إجراء
qs-missing-there-body = وصل وطابق الأصل.
qs-missing-missing-body = تحقق من الملفات الناقصة أو التي لم تكتمل.
qs-missing-note = الملفات التي وصلت محفوظة. الملفات الجديدة أو المتغيرة تحتاج إلى موافقتك مرة أخرى.
qs-missing-ack = إرسال الملفات الناقصة
qs-missing-ack-hint = يتحقق PCTwin من الملفات مرة أخرى، ويقبلها الحاسوب المحمول الآخر مرة أخرى.
qs-missing-complete = وصل كل شيء بالفعل. لم يتبقَّ شيء لإرساله.
qs-missing-unavailable = تعذّر العثور على الإيصال السابق، لذا لا يستطيع PCTwin معرفة ما الناقص. ابدأ إرسالًا جديدًا بدلًا من ذلك.
qs-missing-continue = التحقق مما سيُرسل

## Q20 Stop this send? (and decline)

qs-stop-title = هل تريد إيقاف هذا الإرسال؟
qs-stop-body = تحقق مما وصل بالفعل قبل أن توقف الإرسال.
qs-stop-arrived-title = الملفات التي وصلت كاملة
qs-stop-arrived-body = تبقى في المجلد الذي اخترته.
qs-stop-rest-title = الملفات التي لم تُرسل بعد
qs-stop-rest-body = لا يُرسل أي شيء جديد بعد الإيقاف.
qs-stop-partial-title = الملفات التي وصلت جزئيًا
qs-stop-partial-body = لا تُحفظ أبدًا باسمها الحقيقي، فلا يظنها أحد نسخًا كاملة.
qs-stop-status-keep = الاحتفاظ
qs-stop-status-stop = إيقاف
qs-stop-status-review = مراجعة
qs-stop-ack = إيقاف هذا الإرسال
qs-stop-ack-hint = الملفات التي وصلت كاملة لا تُحذف.
qs-stop-confirm = الإيقاف وعرض الإيصال
qs-stop-keep = متابعة الإرسال
qs-decline-title = هل تريد رفض هذه الملفات؟
qs-decline-body = لن يُحفظ أي شيء على هذا الحاسوب المحمول، وسيُبلَّغ المرسل بأنك رفضت.
qs-decline-confirm = رفض
qs-already-finished = انتهى هذا الإرسال بالفعل. اطّلع على الإيصال لمعرفة ما وصل.

## F08 Your moves, kept on this laptop (history)

history-title = عمليات النقل، محفوظة على هذا الحاسوب المحمول.
history-body = افتح تقريرًا مرة أخرى، أو تابع عملية نقل محفوظة.
history-row-move = ‏{ $from } ← { $to }
history-row-sent = أُرسل إلى { $label }
history-row-received = استُلم من { $label }
history-status-finished = مكتمل
history-status-steps-left =
    { NUMBER($count) ->
        [zero] مكتمل، وتبقى عليك { $count } خطوة
        [one] مكتمل، وتبقى عليك خطوة واحدة
        [two] مكتمل، وتبقى عليك خطوتان
        [few] مكتمل، وتبقى عليك { $count } خطوات
        [many] مكتمل، وتبقى عليك { $count } خطوة
       *[other] مكتمل، وتبقى عليك { $count } خطوة
    }
history-status-paused = متوقف مؤقتًا
history-status-stopped = أُوقف بأمان
history-status-partial = لم يصل كل شيء
history-open-report = التقرير
history-open-receipt = الإيصال
history-continue = متابعة
history-export = تصدير السجل
history-exported = حُفظ السجل في { $path }.
history-export-failed = تعذّر حفظ السجل هناك. اختر مجلدًا آخر وحاول مرة أخرى.
history-note = يبقى هذا السجل على هذا الحاسوب المحمول. لا يُرسل أي شيء إلى أي مكان إلا إذا صدّرته وشاركته بنفسك.
history-empty = ستظهر هنا أول عملية نقل لك وتقريرها.
history-empty-title = لا توجد عمليات نقل مسجلة بعد
history-start = بدء عملية نقل جديدة

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = الإرسال السريع غير مفعّل في هذا الإصدار بعد. لم يُرسل أو يُستلم أي شيء.
qs-tally-counting =
    { NUMBER($count) ->
        [zero] { $count } عنصر محدد · لا يزال الحجم قيد الحساب
        [one] عنصر واحد محدد · لا يزال الحجم قيد الحساب
        [two] عنصران محددان · لا يزال الحجم قيد الحساب
        [few] { $count } عناصر محددة · لا يزال الحجم قيد الحساب
        [many] { $count } عنصرًا محددًا · لا يزال الحجم قيد الحساب
       *[other] { $count } عنصر محدد · لا يزال الحجم قيد الحساب
    }
qs-gone = لم يعد { $name } موجودًا. ربما نُقل أو حُذف. ألغِ تحديده، أو أعده إلى مكانه وأضفه مرة أخرى.
qs-choose-why-problems = لا يمكن إرسال بعض العناصر المحددة بعد. ألغِ تحديدها، أو أصلحها كما هو موضح، للمتابعة.
qs-check-missing = لم يعد { $name } موجودًا. ارجع وأزله، أو أعده إلى مكانه.
qs-check-unreadable = لم يعد PCTwin قادرًا على قراءة { $name }. اسمح بالوصول، أو ارجع وأزله.
qs-check-show-all =
    { NUMBER($count) ->
        [zero] عرض كل الملفات ({ $count })
        [one] عرض الملف
        [two] عرض الملفين
        [few] عرض كل الملفات ({ $count })
        [many] عرض كل الملفات ({ $count })
       *[other] عرض كل الملفات ({ $count })
    }
qs-check-change = تغيير ما سأرسله
qs-land-none = لم يُختر أي مجلد بعد.
qs-land-space-unknown = تعذّر على PCTwin قياس المساحة المتاحة هنا. تحتاج هذه الملفات إلى { $needed }.
qs-conflict-existing-unknown = موجود مسبقًا
qs-conflict-choice-open = لم يُحدَّد بعد
qs-conflict-why = اختر ما يحدث لكل ملف، ثم حدّد المربع، للمتابعة.
qs-approve-folder-title = مجلد الاستلام
qs-status-recipient-choice = اختيار المستلم
qs-sending-card = جارٍ إرسال الملفات المحددة
qs-progress-of = من { $total }
qs-receipt-partial-title = لم يصل كل شيء
qs-receipt-metric-items =
    { NUMBER($count) ->
        [zero] عنصر محدد مستلم
        [one] عنصر محدد مستلم
        [two] عنصران محددان مستلمان
        [few] عناصر محددة مستلمة
        [many] عنصرًا محددًا مستلمًا
       *[other] عنصر محدد مستلم
    }
qs-receipt-metric-size = تم التحقق منها مقارنةً بالمصدر
qs-receipt-sent-as = أُرسل باسم { $path }
qs-receipt-renamed = كان هناك ملف بهذا الاسم، لذا حصل هذا الملف على اسم جديد.
qs-receipt-chip-already = موجود مسبقًا
qs-receipt-already = كان هناك ملف مطابق بالفعل ({ $path })، لذا لم يُرسل مرة أخرى.
qs-receipt-chip-declined = لم يُقبل
qs-receipt-declined-send = لم يقبل الحاسوب المحمول الآخر هذا الملف، لذا لم يُرسل.
qs-receipt-declined-receive = لم تقبل هذا الملف، لذا لم يُحفظ.
qs-receipt-more = مزيد من التفاصيل
qs-receipt-empty = لم يُرسل أي شيء، لذا لا يوجد ما يُعرض.
qs-receipt-already-count =
    { NUMBER($count) ->
        [zero] { $count } عنصر كانت موجودة مسبقًا كملفات مطابقة
        [one] عنصر واحد كان موجودًا مسبقًا كملف مطابق
        [two] عنصران كانا موجودين مسبقًا كملفين مطابقين
        [few] { $count } عناصر كانت موجودة مسبقًا كملفات مطابقة
        [many] { $count } عنصرًا كانت موجودة مسبقًا كملفات مطابقة
       *[other] { $count } عنصر كانت موجودة مسبقًا كملفات مطابقة
    }
qs-missing-new-send = بدء إرسال جديد
history-open-name = { $what }: { $title }

## R05 التراجع: ما سيفعله التراجع على كل محرك أقراص، وما انتهت إليه كل نسخة

undo-here-removes = يحذف التراجع النُسخ التي أنشأها PCTwin ولم تغيّرها. يتم الاحتفاظ بكل ما غيّرته.
undo-here-to-trash = ينقل التراجع النُسخ التي أنشأها PCTwin ولم تغيّرها إلى سلة المهملات. يتم الاحتفاظ بكل ما غيّرته، ولا يُحذف أي شيء.
undo-here-keeps-fat = عند التراجع، سيتم الاحتفاظ بكل نسخة على { $drive }. يستخدم محرك الأقراص هذا FAT أو exFAT، وهما يعيدان استخدام الأرقام التي تميّز الملفات، لذلك لا يستطيع PCTwin أن يتأكد أيّ الملفات تخصّه. يمكنك حذفها بنفسك.
undo-here-keeps-other = عند التراجع، سيتم الاحتفاظ بكل نسخة على { $drive }. لا يستطيع PCTwin نقل الملفات أو حذفها بأمان على هذا النوع من محركات الأقراص. يمكنك حذفها بنفسك.
undo-here-more = مزيد من التفاصيل
undo-here-more-trash = لا يتيح جهاز Mac لـ PCTwin أن يتأكد من أن أي تطبيق آخر لم يعد يكتب في الملف، لذلك لا يحذف PCTwin أي ملف أبدًا. بل ينقل كل نسخة إلى سلة المهملات على محرك الأقراص نفسه. يمكنك إرجاعها من سلة المهملات.
undo-here-more-fat = على محركات الأقراص FAT وexFAT، قد يحصل ملف جديد على الرقم الذي كان لملف محذوف. لا يتعامل PCTwin إلا مع الملفات التي يستطيع إثبات أنها تخصّه وأنها لم تتغيّر، لذلك يتم الاحتفاظ بهذه الملفات.
undo-done-trashed = نُقل إلى سلة المهملات باسم "{ $name }". يمكنك إرجاعه من سلة المهملات.
undo-kept-fat = تم الاحتفاظ به. يعيد محرك الأقراص هذا (FAT أو exFAT) استخدام الأرقام التي تميّز الملفات، لذلك لا يستطيع PCTwin أن يتأكد أن هذه نسخته. إذا لم تكن بحاجة إليها، يمكنك حذفها بنفسك.
undo-kept-no-trash = تم الاحتفاظ به. لم يتمكن PCTwin من نقل هذه النسخة إلى سلة المهملات على محرك الأقراص الخاص بها. إذا لم تكن بحاجة إليها، يمكنك حذفها بنفسك.
