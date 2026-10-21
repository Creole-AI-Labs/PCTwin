# PCTwin screens, zh-CN (Simplified Chinese). Machine-translated ahead of time from ../en/main.ftl;
# a string missing here falls back to English.

app-name = PCTwin
brand-home =
    .aria-label = PCTwin首页
language-label = 语言

footer-free = 个人迁移免费，无需账户。
footer-help = 帮助

## F01 Welcome

welcome-eyebrow = 你的新篇章，准备好了。
welcome-title-line1 = 你的配置。
welcome-title-line2 = 你的下一台笔记本电脑。
welcome-body = 迁移时，把你的文件、应用和熟悉的设置一起带上。
welcome-start = 开始迁移

## F06 Old or new laptop

role-back = 返回
role-eyebrow = 开始迁移
role-title = 这是哪台笔记本电脑？
role-body = 在两台笔记本电脑上都打开PCTwin，然后选择这是哪一台。
role-old-title = 我的旧笔记本电脑
role-old-body = 发送我的文件、应用和设置。
role-new-title = 我的新笔记本电脑
role-new-body = 从另一台笔记本电脑接收我的配置。
role-note = 在你查看并同意方案之前，什么都不会迁移。迁移期间，你的旧笔记本电脑保持原样。

## Frame parts (footer note, Back, steps)

footer-note-move = 个人迁移免费
footer-note-send = 附近发送免费
frame-back = 返回
frame-step-done = 已完成
frame-step-current = 当前
frame-step-todo = 待完成
frame-step-position = 第{ $number }步，共{ $total }步

## F01 Unfinished move

resume-open = 恢复你的迁移

## C01 How will you connect?

connect-eyebrow = 你的迁移
connect-title = 你想怎样连接？
connect-body = 无论选择哪种方式，步骤都一样。
connect-wifi-title = Wi-Fi
connect-wifi-body = 两台笔记本电脑连接同一个Wi-Fi。
connect-wifi-status = 可用
connect-later = 网线、PCTwin桥接线以及通过外接硬盘迁移，将在以后的版本中提供。
connect-usb-note = 普通USB线不能连接两台笔记本电脑来迁移。
connect-use-wifi = 使用Wi-Fi
connect-unit-mbps = { NUMBER($value, maximumFractionDigits: 0) } MB/s
connect-unit-gb = { NUMBER($value, maximumFractionDigits: 1) } GB

## C02 Put both laptops together

connect-prep-title = 把两台笔记本电脑放在一起。
connect-prep-body = 把它们连接到同一个Wi-Fi，然后在每台电脑上打开PCTwin。
connect-prep-step-1-title = 连接同一个Wi-Fi
connect-prep-step-1-body = 复制文件不需要连接互联网。
connect-prep-step-1-network = 网络：{ $network }。
connect-prep-step-2-title = 允许查找附近的笔记本电脑
connect-prep-step-2-body = 如果Windows或macOS请求权限，请选择“允许”。
connect-prep-step-3-title = 在每台笔记本电脑上选择角色
connect-prep-step-3-body = 一台发送，另一台接收。
connect-prep-note = 访客或公共Wi-Fi可能让笔记本电脑无法发现彼此。
connect-prep-continue = 继续配对

## C06 Meet your other laptop

peer-eyebrow = 你的迁移
peer-title = 这是你的另一台笔记本电脑。
peer-body = 两台笔记本电脑都已连接并准备好。
peer-sending = 发送中
peer-receiving = 接收中
peer-old = 旧笔记本电脑
peer-new = 新笔记本电脑
peer-version-label = PCTwin
peer-version = { $version }
peer-status-update = 需要更新
peer-status-unsupported = 等待中
peer-ready-note = 两台笔记本电脑配对后，PCTwin才会列出你的文件。如果版本不同，请在两台电脑上都更新PCTwin。
peer-version-differs = 这两台笔记本电脑上的PCTwin版本不同。请在两台电脑上都更新PCTwin后继续。
peer-version-old-body = 版本不同：{ $version }。
peer-version-new-body = 两台笔记本电脑需要相同的版本。
peer-update-title = 更新PCTwin
peer-update-body = 让两台笔记本电脑安装相同的版本。
peer-direction-unsupported = 这个版本的PCTwin暂时还不能在这两种系统之间迁移。
peer-continue = 检查连接

## C07 Let’s check the connection

check-eyebrow = 你的迁移
check-title = 我们来检查连接。
check-body = PCTwin会检查速度、空间和电源，以便规划迁移并估算时间。
check-measuring = 正在测量连接……
check-speed-label = 连接速度
check-space-label = 新笔记本电脑上可用
check-link-weak = 信号弱，迁移可能较慢。把笔记本电脑移近Wi-Fi路由器会有帮助。
check-power-title = 电源
check-power-both = 两台笔记本电脑都已连接电源。
check-power-battery = { $label }正在使用电池。请插上电源，以免长时间的迁移被打断。
check-space-title = 存储空间
check-space-body = 新笔记本电脑上有{ $free }可用空间。在迁移开始前，你会看到你的选择是否放得下。
check-internet-title = 互联网
check-internet-on = 已连接。复制文件不需要互联网。
check-internet-off = 未连接。没关系，复制文件不需要互联网。
check-status-ready = 已准备好
check-status-plug-in = 插上电源
check-status-connected = 可用
check-status-not-needed = 不需要
check-status-unknown = 未知
check-speed-unknown = PCTwin这次没能测量连接。请稍后再次检查。
check-space-unknown = PCTwin无法读取新笔记本电脑上的可用空间。
check-power-unknown = PCTwin无法判断{ $label }是否插上了电源。
check-internet-unknown = PCTwin无法判断这台笔记本电脑是否在线。没关系，复制文件不需要互联网。
check-again = 再次检查
check-estimate-note = 预计时间仅供参考，并非固定承诺。
check-continue = 选择要迁移的内容

## C09 Waiting for your other laptop

reconnect-title = 正在等待你的另一台笔记本电脑。
reconnect-body = 目前已复制并检查的内容都会保留。
reconnect-card-title = 连接中断，迁移已暂停
reconnect-progress =
    .aria-label = 连接中断，迁移已暂停
reconnect-saved = 目前已复制并检查{ $done }，其余部分稍后继续。
reconnect-last-seen = 上次在线：{ $ago }。
reconnect-status-offline = 离线
reconnect-status-trying = 正在重新连接
reconnect-status-pending = 等待中
reconnect-trying = 正在自动重新连接……请让两台笔记本电脑保持开机、不要休眠，并连接同一个Wi-Fi。
reconnect-expired = 时间太久，无法自动重新连接。请重新配对，PCTwin会检查哪些内容已经复制过。
reconnect-unavailable = 无法连接到{ $label }。请检查它是否已开机、没有休眠、打开了PCTwin，并连接同一个Wi-Fi。
reconnect-try-now = 重试连接
reconnect-pair-again = 重新配对
reconnect-details = 安全地重新连接
reconnect-details-body = 只有这两台笔记本电脑可以重新连接，而且每次都使用新的私密连接。

## C10 The connection has ended (dialog)

ended-title = 连接已结束。
ended-body = 你的迁移已保存。旧的配对码已失效。
ended-kept-title = 已保留
ended-kept-body = 你的迁移记录，以及所有已复制并检查的内容。
ended-not-kept-title = 未保留
ended-not-kept-body = 无需重新配对即可继续的权限。
ended-pair-again = 重新配对
ended-pair-again-body = 重新连接后，PCTwin会检查哪些内容已经复制过。
ended-see-moved = 打开已保存的迁移
ended-see-moved-body = 查看目前已复制并检查的内容。
ended-home = 返回首页

## Pairing, shared

pair-back = 返回
pair-try-again = 重试

## C03 Old laptop: show the code

pair-old-eyebrow = 旧笔记本电脑
pair-old-title = 我们来连接你的笔记本电脑。
pair-old-body = 在你的新笔记本电脑上输入下面的配对码。
pair-old-next = 接下来，你要核对两台笔记本电脑上显示的同一个数字。
pair-old-network = 让两台笔记本电脑保持连接同一个Wi-Fi。
pair-old-waiting = 正在等待你的新笔记本电脑……
pair-old-starting = 正在准备配对码……
pair-old-label-title = 这台笔记本电脑的临时名称
pair-old-label-hint = 你的新笔记本电脑会通过这个名称找到这台电脑。
pair-old-shuffle = 换一个名称
pair-old-rename = 输入名称
pair-old-rename-label = 这台笔记本电脑的名称
pair-old-rename-note = 这个Wi-Fi上的任何人都能看到这个名称。
pair-old-rename-save = 使用这个名称
pair-old-rename-cancel = 取消
pair-old-code-meta =
    { $seconds ->
       *[other] 每分钟更换一次。{ $seconds }秒后出现新配对码。
    }
pair-old-steps-title = 在你的新笔记本电脑上
pair-old-step-1 = 打开PCTwin，选择“我的新笔记本电脑”。
pair-old-step-2 = 如果出现列表，请选择这台笔记本电脑的名称。
pair-old-step-3 = 输入这里显示的六位配对码。
pair-old-wait-title = 新配对码马上就来。
pair-old-wait-body =
    { $seconds ->
       *[other] 这次尝试没有成功。{ $seconds }秒后会出现新配对码。
    }
pair-old-locked-title = 配对已停止。
pair-old-locked-body = 错误尝试次数过多，为保护这台笔记本电脑的安全，配对已停止。没有共享任何内容。准备好后请重新开始。
pair-old-start-again = 重新开始
pair-old-timed-out = 没有及时选择数字，这是新的配对码。
pair-old-declined = 数字不匹配，因此没有共享任何内容。这是新的配对码。
pair-old-left = 在选择数字之前，新笔记本电脑停止了配对，因此没有共享任何内容。这是新的配对码。
pair-network-title = 选择你的Wi-Fi。
pair-network-hint = 选择你的新笔记本电脑所连接的网络。

## C05 Old laptop: match the number

pair-choose-eyebrow = 确认连接
pair-choose-title = 选择相同的数字。
pair-choose-body = 看看你的新笔记本电脑，选择它显示的数字。
pair-choose-number = 数字{ $number }
pair-choose-confirm = 确认这两台笔记本电脑
pair-choose-renewed = 已显示新的数字。请选择你的新笔记本电脑上显示的那个。
pair-choose-none = 都不匹配
pair-choose-note = 如果新笔记本电脑没有显示数字，或者这些数字都不匹配，请选择“都不匹配”。
pair-choose-safe = 目前还没有共享任何内容。
pair-choose-checking = 正在检查……

## C04 New laptop: find the old laptop and enter the code

pair-new-eyebrow = 新笔记本电脑
pair-new-searching-title = 正在搜索你的旧笔记本电脑……
pair-new-searching-body = 请在旧笔记本电脑上保持PCTwin打开，并与这台电脑连接同一个Wi-Fi。
pair-new-list-title = 哪一台是你的旧笔记本电脑？
pair-new-list-body = 选择你的旧笔记本电脑屏幕上显示的名称。
pair-new-same-label = 附近另一台笔记本电脑也叫这个名称。请查看旧笔记本电脑的屏幕。
pair-new-none-title = 没有找到旧笔记本电脑。
pair-new-none-body = 请检查旧笔记本电脑上的PCTwin是否已打开并显示配对码，以及两台电脑是否连接同一个Wi-Fi。然后重新搜索。
pair-new-search-again = 重新搜索
pair-new-code-title = 输入配对码。
pair-new-code-body = 输入{ $label }上显示的配对码。
pair-new-code-label = 六位配对码
pair-new-code-hint = 看你的旧笔记本电脑屏幕上的配对码。它每分钟更换一次。
pair-new-connect = 连接
pair-new-connecting = 正在连接……
pair-new-other-laptop = 选择另一台笔记本电脑
pair-new-number-eyebrow = 确认连接
pair-new-number-title = 在你的旧笔记本电脑上选择这个数字。
pair-new-number-tile = 你的数字
pair-new-number-label = 你的数字是{ $number }
pair-new-number-body = 你的旧笔记本电脑会显示三个数字，请在那里选择这一个。
pair-new-number-waiting = 正在等待你的旧笔记本电脑……

## Paired (both laptops)

pair-done-eyebrow = 一切就绪
pair-done-title = 你的笔记本电脑已配对。
pair-done-body = 你的两台笔记本电脑已连接。下一步是选择要迁移的内容，这部分还在开发中，所以目前还没有迁移任何东西。

## Pairing problems (each says what happened, then what to do)

pairing-code-invalid = 配对码需要六位数字。请查看旧笔记本电脑的屏幕，然后重新输入。
pairing-code-wrong = 配对码不正确。请查看旧笔记本电脑的屏幕，然后再次尝试。
pairing-code-expired = 配对码已更换。请输入旧笔记本电脑屏幕上的新配对码。
pairing-busy = 旧笔记本电脑正在与另一台电脑配对。请稍等片刻，然后再次尝试。
pairing-wait = 旧笔记本电脑正在准备新的配对码。请等待它准备好，然后输入新配对码。
pairing-locked = 错误尝试次数过多，配对已停止。请在旧笔记本电脑上选择“重新开始”。
pairing-unreachable = 这台笔记本电脑无法连接到旧笔记本电脑。请检查两台电脑是否连接同一个Wi-Fi，然后再次尝试。
pairing-wrong-number = 这个数字与新笔记本电脑上的不匹配。没有共享任何内容。请在新电脑上输入新的配对码，再次尝试。
pairing-no-match = 旧笔记本电脑上的数字不匹配。没有共享任何内容。请输入旧电脑屏幕上的新配对码。
pairing-timed-out = 没有及时选择数字。请使用旧笔记本电脑上的新配对码再次尝试。
pairing-connection-lost = 两台笔记本电脑之间的连接已断开。没有共享任何内容。请再次尝试。
pairing-connection-lost-new = 与旧笔记本电脑的连接已断开。没有共享任何内容。
pairing-different-version = 两台笔记本电脑上的PCTwin版本不同。请在两台电脑上都更新PCTwin，然后再次尝试。
pairing-name-invalid = 这个名称不能使用。请换一个更短的名称，只用字母和数字。
pairing-no-network = 这台笔记本电脑没有连接Wi-Fi或有线网络。请连接网络，然后再次尝试。
pairing-choose-network = 这台笔记本电脑连接了多个网络。请选择你的新笔记本电脑所连接的那个：
pairing-failed = 这次配对没有成功。没有共享任何内容。请再次尝试。

## M06 Match people to their profiles (Who’s moving?)

people-eyebrow = 你的迁移
people-title = 为每个人匹配对应的账户。
people-body = 选择每个人的文件放到新笔记本电脑上的哪里。
people-col-old = 旧笔记本电脑
people-col-new = 新笔记本电脑
people-col-status = 状态
people-you = 你
people-select =
    .aria-label = { $name }在新笔记本电脑上的账户
people-option-existing = { $account } / 新笔记本电脑
people-option-new = 创建新账户
people-option-leave-out = 不包括
people-option-choose = 请选择……
people-new-account-hint = 新笔记本电脑会在创建账户之前请求权限。
people-suggested = 因为名称相同而推荐。请确认是同一个人。
people-suggested-use = 使用{ $account }
people-status-ready = 已确认
people-status-waiting = 需要对方同意
people-status-left-out = 未包括
people-status-choose = 需要你选择
people-status-changed = 已由你更改
people-waiting-body = { $name }需要在旧笔记本电脑上登录并同意，其文件才能迁移。在此之前，这些文件不会被改动。
people-just-me = 在旧笔记本电脑上只找到了你自己的账户。
people-changed-note = 你更改了某人文件的去向。继续之前请检查一下。
people-shared-title = 共享项目
people-shared-body = 属于旧笔记本电脑上所有人的共享文件夹和公共文档。
people-shared-select =
    .aria-label = 共享项目的去向
people-shared-option-shared = 新笔记本电脑上的共享文件夹
people-shared-option-person = 和{ $name }的文件放在一起
people-shared-option-leave-out = 不包括
people-note = PCTwin绝不会更改任何人的密码，也不会绕过他人的登录。
people-list-label = 旧笔记本电脑上的人
people-todo =
    { $count ->
        [0] 每个可以迁移的人都有了去处。
       *[other] 还有{ $count }个人需要你选择。
    }
people-waiting-count =
    { $count ->
       *[other] 有{ $count }个人正在等待同意。你可以继续；在对方同意之前，其文件不会被改动。
    }
people-continue = 继续

## M26 Before you leave this laptop (things to do on the old laptop)

before-eyebrow = 你的迁移
before-title = 在离开这台笔记本电脑之前。
before-body = 趁旧笔记本电脑还在手边，检查一下这些事项。
before-licences-title = 应用许可证
before-licences-body = 有些应用需要你自己转移许可证。
before-passwords-title = 账户找回
before-passwords-body = 确保你仍然能登录你的账户，并能打开任何上了锁的硬盘。
before-codes-title = 把登录验证码放在手边
before-codes-body = 如果某个账户会给你发验证码，或者用应用确认是你本人，请把那部手机或应用放在身边。
before-online-title = 未保存的工作
before-online-body = 保存所有草稿，并检查在线存储的文件是否是最新的。
before-done =
    .aria-label = 我已完成：{ $item }
before-note = 不要在这里输入密码或恢复密钥。完成这份列表并不代表旧笔记本电脑可以安全清空。
before-empty = 迁移之前，旧笔记本电脑上没有需要处理的事项。
before-continue = 返回迁移方案

## S11 Get help with this task (a short problem code)

help-eyebrow = 帮助和隐私
help-title = 获取这项任务的帮助。
help-body = 分享一个简短的问题码，让别人知道出了什么问题。请先查看它包含哪些内容。
help-making = 正在生成问题码……
help-code-title = 问题码
help-code =
    .aria-label = 问题码{ $code }
help-contains-title = 包含的内容
help-contains-problem = 问题的类型，以及任务进行到了哪一步。
help-contains-step = 问题发生在迁移的哪一步。
help-contains-versions = PCTwin版本，以及每台笔记本电脑运行的系统，例如Windows或macOS。
help-never-title = 绝不包含的内容
help-chip-included = 包含
help-chip-excluded = 绝不包含
help-never-names = 你或任何其他人的名字。
help-never-files = 你的文件内容，或任何密码。
help-never-network = 文件或文件夹名称、文件存放的地方，或Wi-Fi名称。
help-note = 这个问题码不会让任何人访问你的笔记本电脑。
help-copy = 复制问题码
help-copied = 问题码已复制。
help-share = 分享
help-share-hint = 通过WhatsApp或任何应用发送。
help-share-text = 我的PCTwin问题码是{ $code }。
help-failed = 无法生成问题码。请再次尝试。如果还是不行，请改用你自己的话描述问题。
help-try-again = 重试
help-copy-failed = 无法复制问题码。请选中上面的问题码，自己复制。

## Label words (the name each laptop shows while pairing)

label-colour-red = 红色
label-colour-orange = 橙色
label-colour-yellow = 黄色
label-colour-green = 绿色
label-colour-blue = 蓝色
label-colour-purple = 紫色
label-colour-pink = 粉色
label-colour-brown = 棕色
label-colour-black = 黑色
label-colour-white = 白色
label-colour-grey = 灰色
label-colour-gold = 金色

label-animal-ant = 蚂蚁
label-animal-badger = 獾
label-animal-bat = 蝙蝠
label-animal-bear = 熊
label-animal-bee = 蜜蜂
label-animal-beetle = 甲虫
label-animal-bison = 野牛
label-animal-butterfly = 蝴蝶
label-animal-camel = 骆驼
label-animal-cat = 猫
label-animal-cheetah = 猎豹
label-animal-chicken = 母鸡
label-animal-crab = 螃蟹
label-animal-crane = 仙鹤
label-animal-crocodile = 鳄鱼
label-animal-crow = 乌鸦
label-animal-deer = 鹿
label-animal-dolphin = 海豚
label-animal-donkey = 驴
label-animal-dove = 鸽子
label-animal-dragonfly = 蜻蜓
label-animal-duck = 鸭子
label-animal-eagle = 老鹰
label-animal-elephant = 大象
label-animal-falcon = 猎鹰
label-animal-flamingo = 火烈鸟
label-animal-fox = 狐狸
label-animal-frog = 青蛙
label-animal-gazelle = 羚羊
label-animal-swordfish = 剑鱼
label-animal-giraffe = 长颈鹿
label-animal-goat = 山羊
label-animal-goose = 鹅
label-animal-gorilla = 大猩猩
label-animal-hamster = 仓鼠
label-animal-hawk = 苍鹰
label-animal-hedgehog = 刺猬
label-animal-armadillo = 犰狳
label-animal-hippo = 河马
label-animal-horse = 马
label-animal-hummingbird = 蜂鸟
label-animal-jaguar = 美洲豹
label-animal-jellyfish = 水母
label-animal-kangaroo = 袋鼠
label-animal-kingfisher = 翠鸟
label-animal-koala = 考拉
label-animal-ladybird = 瓢虫
label-animal-lemur = 狐猴
label-animal-leopard = 豹子
label-animal-lion = 狮子
label-animal-lizard = 蜥蜴
label-animal-llama = 羊驼
label-animal-lobster = 龙虾
label-animal-lynx = 猞猁
label-animal-macaw = 金刚鹦鹉
label-animal-meerkat = 狐獴
label-animal-mole = 鼹鼠
label-animal-moose = 驼鹿
label-animal-pufferfish = 河豚
label-animal-mouse = 老鼠
label-animal-octopus = 章鱼
label-animal-ostrich = 鸵鸟
label-animal-otter = 水獭
label-animal-owl = 猫头鹰
label-animal-panda = 熊猫
label-animal-kiwi = 几维鸟
label-animal-parrot = 鹦鹉
label-animal-peacock = 孔雀
label-animal-pelican = 鹈鹕
label-animal-penguin = 企鹅
label-animal-puffin = 海鹦
label-animal-rabbit = 兔子
label-animal-raccoon = 浣熊
label-animal-rhino = 犀牛
label-animal-robin = 知更鸟
label-animal-salmon = 三文鱼
label-animal-seahorse = 海马
label-animal-seal = 海豹
label-animal-shark = 鲨鱼
label-animal-sheep = 绵羊
label-animal-sloth = 树懒
label-animal-snail = 蜗牛
label-animal-sparrow = 麻雀
label-animal-squirrel = 松鼠
label-animal-starfish = 海星
label-animal-stork = 白鹳
label-animal-swan = 天鹅
label-animal-tiger = 老虎
label-animal-snake = 蛇
label-animal-toucan = 巨嘴鸟
label-animal-turtle = 海龟
label-animal-walrus = 海象
label-animal-whale = 鲸鱼
label-animal-wolf = 狼
label-animal-woodpecker = 啄木鸟
label-animal-yak = 牦牛
label-animal-zebra = 斑马
label-animal-dog = 狗
label-animal-cow = 奶牛
label-animal-beaver = 河狸

label-picked = { $colour }{ $animal }
label-unnamed = 未命名的笔记本电脑

## M02 Finding your files

scan-eyebrow = 你的迁移
scan-title = 正在查找你熟悉的配置。
scan-body = PCTwin找到的内容会被保存，所以搜索停止后也能继续。
scan-progress =
    .aria-label = 正在搜索你选择的地方
scan-found =
    { NUMBER($count) ->
       *[other] 目前已找到{ $count }个文件。
    }
scan-step-usual-title = 文件和文件夹
scan-step-usual-body = 已搜索PCTwin可以读取的地方。
scan-step-other-title = 其他文件夹和硬盘
scan-step-other-body = 你在常用地方以外使用的文件夹和硬盘。
scan-step-protected-title = 受保护的地方
scan-step-protected-body =
    { NUMBER($count) ->
       *[other] 还有{ $count }个地方需要你的权限。
    }
scan-step-done = 已完成
scan-step-current = 当前
scan-step-todo = 待完成
scan-unreadable-note = PCTwin无法读取的地方会留在列表中，不算作已搜索。
scan-status-not-searched = 未搜索
scan-paused = 搜索已暂停。目前找到的所有内容都已保存。
scan-resumed = 搜索已从停止的地方继续。
scan-partial =
    { NUMBER($count) ->
       *[other] 还有{ $count }个受保护的地方尚未搜索。
    }
scan-allow = 允许访问
scan-pause = 暂停
scan-resume = 继续搜索
scan-continue = 查看找到的内容

## M03 Everything you chose, in view

overview-eyebrow = 你的迁移
overview-title = 你选择的所有内容，一目了然。
overview-body = 在PCTwin制定方案之前，检查它找到的内容。
overview-size-label = 已选文件
overview-items-label =
    { NUMBER($count) ->
       *[other] 个文件
    }
overview-attention-label =
    { NUMBER($count) ->
       *[other] 需要你留意
    }
overview-files-title = 文件和文件夹
overview-files-body = 你选择的文档、照片和其他文件夹。
overview-dest-title = 去向
overview-dest-body = 每个文件夹在新笔记本电脑上存放的地方。
overview-people-title = 人员
overview-people-body = 为每个人选择新笔记本电脑上的账户。
overview-review = 查看
overview-partial =
    { NUMBER($count) ->
       *[other] 还有{ $count }个受保护的地方尚未搜索。
    }
overview-allow = 允许访问
overview-continue = 继续

## M04 Keep what matters to you

files-eyebrow = 你的迁移
files-title = 保留对你重要的内容。
files-body = 选择文件夹、其他硬盘以及任何其他地方。
files-search-label = 查找文件夹
files-search-placeholder = 搜索文件夹
files-select =
    .aria-label = 迁移{ $name }
files-folder-detail =
    { NUMBER($count) ->
       *[other] { $size } · { $count }个文件
    }
files-status-protected = 需要权限
files-protected-body = PCTwin需要你的权限才能读取这个文件夹。在你允许之前，其中的文件不计入统计。
files-allow = 允许访问
files-status-added = 由你添加
files-add = 添加其他文件夹
files-remove =
    .aria-label = 移除{ $name }
files-no-limit = 免费迁移对文件大小和文件夹数量都没有限制。
files-total =
    { NUMBER($count) ->
       *[other] 已选择{ $count }个文件夹，共{ $size }
    }
files-none = 至少选择一个文件夹才能继续。
files-continue = 选择目标位置

## M05 Give every file a home

dest-eyebrow = 你的迁移
dest-title = 给每个文件安个家。
dest-body = 选择每个文件夹放到新笔记本电脑上的哪里。
dest-col-chosen = 你选择的内容
dest-col-place = 去向
dest-col-placement = 硬盘
dest-placement-local = 内置
dest-placement-external = 外接
dest-col-status = 状态
dest-select =
    .aria-label = { $name }的目标位置
dest-option-folder = 新笔记本电脑 / { $folder }
dest-option-drive = 外接硬盘 / { $drive }
dest-option-choose = 选择文件夹
dest-default-label = 默认位置
dest-status-ready = 已准备好
dest-status-missing = 硬盘未连接
dest-status-space = 空间不足
dest-all-ready = 每个文件夹在新笔记本电脑上都有了存放的地方。
dest-missing = { $drive }没有连接到新笔记本电脑。请连接它，或为这些文件选择其他地方。
dest-space = { $place }有{ $free }可用空间，但这些文件需要{ $needed }。请在那里腾出空间、选择其他地方，或减少要迁移的文件。
dest-change = 更改
dest-continue = 查看方案

## M23 Approve the move you want

plan-eyebrow = 你的迁移
plan-title = 同意你想要的迁移方案。
plan-body = PCTwin只做这个方案里写的事。
plan-what-title = 你选择的内容
plan-what-body = 来自你选择的文件夹的{ $size }文件，将放到你挑选的地方。
plan-where-body =
    { NUMBER($count) ->
       *[other] 放到新笔记本电脑上的{ $count }个地方。
    }
plan-hand-title = 开始之前
plan-hand-body =
    { NUMBER($count) ->
       *[other] { $count }个步骤。它们会列在最后的报告中。
    }
plan-hand-none = 没有（就PCTwin目前所知）。
plan-old-safe = 你的旧笔记本电脑保持原样。文件只会被复制，绝不会从旧电脑上移走。
plan-edit = 更改方案
plan-edit-note = 如果你更改方案，需要在任何新内容开始之前重新同意。
plan-approve = 我同意这个方案，包括需要我自己完成的事项。
plan-not-approved = 你还没有同意这个方案，所以不会迁移任何内容。
plan-changed = 你更改了方案，所以在迁移任何新内容之前，请重新同意。
plan-start = 开始迁移

## Plan screens: shared words and the lines the copy deck did not cover

plan-back = 返回
plan-number = { NUMBER($n) }
plan-size-b = { NUMBER($n) }字节
plan-size-kb = { NUMBER($n) } KB
plan-size-mb = { NUMBER($n) } MB
plan-size-gb = { NUMBER($n) } GB
plan-size-tb = { NUMBER($n) } TB
plan-size-at-least = 至少{ $size }
plan-size-unknown = 大小未知
plan-role-home = 主文件夹
plan-role-desktop = 桌面
plan-role-documents = 文档
plan-role-downloads = 下载
plan-role-pictures = 图片
plan-role-music = 音乐
plan-role-videos = 视频
plan-role-public = 公共文件夹
plan-role-app-data = 应用数据
plan-role-other = 其他文件夹
scan-places = 已搜索{ $done }/{ $total }个地方
scan-card-title = 正在搜索你选择的地方
scan-pausing = 处理完当前文件夹后暂停。目前找到的所有内容都已保存。
overview-row-attention = 需要留意
files-status-work = 工作或学校
files-work-body = 这个文件夹由你的工作单位或学校管理，所以除非你勾选它，否则不会包括在内。
files-online-only =
    { NUMBER($count) ->
       *[other] 有{ $count }个文件仅在线存储，所以不会被复制。
    }
files-no-match = 没有与这个名称匹配的文件夹。
dest-status-none = 选择地方
dest-status-readonly = 无法写入
dest-status-format = 文件太大
dest-none = 请为{ $name }在新笔记本电脑上选择一个存放的地方。
dest-readonly = { $place }是只读的，无法复制任何内容到上面。请选择其他地方。
dest-format = 这些文件夹中有一个文件太大，按{ $place }目前的设置方式放不下。请选择其他地方。
plan-place = { $drive }上的{ $folder }
plan-group-to = 放到{ $place }
plan-version = 方案版本{ $revision }
plan-details = 更多详情
plan-reference = 参考编号{ $reference }
plan-hand-cloud-only =
    { NUMBER($count) ->
       *[other] 有{ $count }个文件仅在线存储。PCTwin不会下载它们。请先在旧笔记本电脑上让它们可用，或者跳过它们。
    }
plan-hand-unreadable =
    { NUMBER($count) ->
       *[other] 有{ $count }个项目无法打开，所以不会迁移。请允许访问，或者跳过它们。
    }
plan-hand-work-managed =
    { NUMBER($count) ->
       *[other] 有{ $count }个项目由你的工作单位或学校管理，已被排除。勾选所在的文件夹即可包括它们。
    }
plan-blocked-someone-unmapped = 旧笔记本电脑上有人还没有选择新电脑上的账户。请选择每个人的文件放到哪里，然后再回来。
plan-blocked-account-used-twice = 有两个人会被放进新笔记本电脑上的同一个账户。请为每个人分配各自的账户。
plan-blocked-invalid = 这个方案目前无法同意。请返回检查你的选择。
plan-blocked-fix = 选择账户

## Tray menu

tray-show = 显示PCTwin
tray-quit = 退出PCTwin

## F01 secondary actions and their coming-soon pages

welcome-restore = 还原配置
welcome-rescue = 救援笔记本电脑
welcome-quick-send = 快速发送

soon-eyebrow = 即将推出
soon-restore-title = 还原配置
soon-restore-body = 用你在迁移后保存的配置，重建一台笔记本电脑。这项功能将在以后的版本中推出。
soon-rescue-title = 救援笔记本电脑
soon-rescue-body = 把无法启动的笔记本电脑的硬盘连接到一台正常的电脑上，找回其中的文件。这项功能将在以后的版本中推出。

## X01 Move progress, compact

progress-eyebrow = 你的迁移
progress-title = 正在帮你安顿下来。
progress-title-paused = 迁移已暂停
progress-paired = 已配对
progress-percent =
    .aria-label = 迁移进度
progress-amount = { $done } / { $total }
progress-time-left = 大约还剩{ $range }
progress-time-measuring = 正在估算剩余时间……
progress-speed = { $speed }（实测）
progress-count-needs-action =
    { NUMBER($count) ->
       *[other] { $count }个项目需要你处理
    }
progress-current-copying = 正在复制：{ $name }
progress-current-checking = 正在检查：{ $name }
progress-needs-action =
    { NUMBER($count) ->
       *[other] { $count }个项目需要你处理。其余部分继续迁移。
    }
progress-see-needs-action = 查看需要你处理的项目
progress-interrupted = 与另一台笔记本电脑的连接已断开。PCTwin正在尝试重新连接。已复制并检查的内容都不会丢失。
progress-low-battery = { $label }的电池电量不足。请插上电源，以免迁移被打断。
progress-old-safe = 你的旧笔记本电脑保持原样。
progress-pause = 暂停迁移
progress-compact = 精简显示
progress-expand = 查看全部迁移内容
progress-stop = 停止迁移

## X02 Move progress, expanded

progress-all-title = 查看全部迁移内容。
progress-all-body = 每个项目都会显示它来自哪里，以及当前进展。
progress-metric-overall = 总进度
progress-metric-committed = 已复制并检查
progress-metric-estimate = 预计
progress-col-item = 项目
progress-col-from = 来源
progress-col-status = 状态
progress-state-waiting = 等待中
progress-state-copying = 正在复制
progress-state-verified = 已检查
progress-state-checking = 正在检查
progress-state-paused = 已暂停
progress-checked-note = 项目只有在检查之后才算已复制。

## F09 Mini panel

mini-title = 你的迁移正在附近进行。
mini-title-paused = 迁移已暂停。
mini-what = 正在迁移你选择的文件
mini-progress =
    .aria-label = 迁移进度
mini-pause = 暂停
mini-resume = 恢复
mini-open = 打开迁移
mini-title-send = 你的文件正在发送中。
mini-what-send = 正在发送选择的文件
mini-open-send = 打开发送
mini-progress-send =
    .aria-label = 发送进度

## X05 Paused at a safe point

pause-title = 已在安全点暂停。
pause-title-finishing = 正在暂停到安全点……
pause-body = 已复制并检查的内容都会保留，其余的会等待。
pause-finishing = 先完成{ $name }，以免它只复制了一半。
pause-kept-title = 文件
pause-kept-body = 目前已复制并检查的内容都会保留。
pause-waiting-title = 等待中
pause-waiting-body =
    { NUMBER($count) ->
       *[other] 还有{ $count }个项目要复制。没有丢失任何内容。
    }
pause-status-kept = 已暂停
pause-status-waiting = 等待中
pause-note = 暂停不会撤销任何操作。你的旧笔记本电脑保持原样。
pause-resume = 恢复已保存的迁移

## X10 Stop the remaining move? (danger dialog)

cancel-title = 停止剩余的迁移？
cancel-body = 停止后，已复制并检查的内容都会保留。这不会撤销迁移。
cancel-copied-title = 已复制的文件
cancel-copied-body = 保留在新笔记本电脑上。
cancel-rest-title = 剩余的迁移
cancel-rest-body = 不会开始任何新内容。尚未复制的文件只保留在旧笔记本电脑上。
cancel-partial-title = 只复制了一部分的文件
cancel-partial-body = 绝不会被当作完整文件保留。
cancel-old-title = 旧笔记本电脑
cancel-old-body = 保持原样。
cancel-ack = 我明白停止不会撤销已复制的内容
cancel-stop = 停止并查看报告
cancel-keep = 继续迁移

## R01 Welcome to your next laptop (outcome)

outcome-eyebrow = 迁移结果
outcome-title = 欢迎使用你的新笔记本电脑。
outcome-body = 以下是你同意的每项内容的结果。
outcome-files-title = 已选文件
outcome-status-partial = 部分完成
outcome-next-title = 接下来要做什么
outcome-old-safe = 未完成的内容会一直列在这里。旧笔记本电脑上没有删除任何东西。
outcome-report = 查看完整报告
outcome-home = 返回首页

## R02 Your move, item by item (detailed report)

report-title = 你的迁移，逐项查看。
report-body = 每个项目的结果、原因，以及它的来源和去向。
report-filter-label = 显示
report-col-item = 项目
report-col-result = 结果
report-col-details = 检查或下一步
report-from = 来源：{ $path }
report-to = 去向：{ $path }
report-none = 没有项目是这个结果。
report-more-details = 更多详情

evidence-read-back = 已从硬盘读回进行检查。
evidence-memory = 通过这台笔记本电脑的内存检查，而不是硬盘。
evidence-memory-explain = 副本已读回并且一致，但读取经过了这台笔记本电脑的内存，所以读到的内容可能来自内存而不是硬盘。硬盘本身的故障可能不会在这里显示，所以打开一次文件是个不错的检查方法。
evidence-not-checked = 未检查，因为没有完成。
# Machine-translated 16 October 2026.
why-memory-cannot-skip-cache = 这个硬盘不允许PCTwin直接读取它，所以检查是经过这台笔记本电脑的内存完成的。有些网络硬盘和USB闪存盘就是这样。
why-memory-not-shown = 这个硬盘无法证明PCTwin的读取真的到达了它，所以这次检查算作经过这台笔记本电脑的内存完成。
why-memory-no-way = 这台笔记本电脑的系统没有办法直接读取这个硬盘，所以检查是经过它的内存完成的。
why-memory-stored-specially = 这个文件以特殊方式存储，比如经过压缩或加密，所以系统通过内存读取它。
why-memory-too-small = 这个文件很小，所以硬盘把它和自己的记录放在一起，系统通过内存读取它。

reason-not-enough-space = 新笔记本电脑空间不足。请在上面腾出空间，然后开始一次包含这个项目的新迁移。
reason-changed-during-move = 复制期间，这个文件在旧笔记本电脑上一直在变化。请关闭正在使用它的应用，然后开始一次包含它的新迁移。
reason-unreadable = 旧笔记本电脑不允许PCTwin读取这个项目。请检查它在旧电脑上能否打开，然后开始一次包含它的新迁移。
reason-not-verified = 副本与原文件不一致，所以没有保留。请开始一次包含它的新迁移。如果再次失败，原文件可能已损坏。
reason-refused-by-safety-check = 由于名称、位置或大小的原因，新笔记本电脑的安全检查拒绝了这个项目。原文件仍在旧电脑上，如有需要，你可以用其他方式复制它。
reason-name-not-allowed = 这个名称在新笔记本电脑上不能使用，而且无法生成安全的新名称。请在旧电脑上重命名，然后开始一次包含它的新迁移。
reason-install-failed = 这个应用没有安装成功。请从开发者的官网安装。
reason-no-equivalent = 这个应用没有适用于新笔记本电脑系统的版本。请找一个能在那里使用的类似应用。
reason-sign-in = 已放到位，但你需要登录。请打开它并登录。
reason-activation = 应用已放到位，但需要在新笔记本电脑上使用许可证。请打开它，用你的许可证或账户激活。
reason-permission = 新笔记本电脑需要先授予权限。请打开这个项目，在询问时选择“允许”。
reason-online-only = 这个文件仅在线存储，所以旧笔记本电脑上没有可复制的文件。请在新电脑上，通过在线存储它的应用下载。
reason-manual-install = 这个应用必须从开发者的官网获取。请到那里下载并安装。
reason-restart = 新笔记本电脑需要重启才能完成。准备好后请重启。
reason-finish-by-hand = PCTwin无法替你完成这一步。请按照这份报告中为它列出的步骤操作。

## Item results (every item ends with one of these)

status-copied = 已复制
status-reinstalled = 已重新安装
status-converted = 已转换
status-needs-action = 需要处理
status-skipped-by-user = 已由你跳过
status-failed = 失败

## Screens: words the copy deck did not cover (same voice)

progress-minutes = { $low }到{ $high }分钟
# Machine-translated 16 October 2026.
progress-minutes-one =
    { NUMBER($minutes) ->
       *[other] { $minutes }分钟
    }
progress-speed-range = { $low }到{ $high } { $unit }
progress-count-failed =
    { NUMBER($count) ->
       *[other] { $count }个项目无法复制
    }
progress-queue-empty = 还没有开始任何内容。
progress-approved-job = 已同意的迁移
progress-route-old = 旧笔记本电脑
progress-route-new = 新笔记本电脑
progress-panel-pausing = 正在暂停
outcome-metric-size = 已复制并检查
outcome-metric-action = 需要你处理
progress-percent-unknown = PCTwin仍在计算这次迁移的大小。
mini-title-send-paused = 发送已暂停。
cancel-ack-hint = 勾选上面的方框，即可启用停止按钮。
progress-state-placing = 正在放到位
progress-state-existing = 已存在
outcome-stopped-title = 你的迁移已停止。
outcome-stopped-body = 你停止了迁移。在此之前已复制并检查的内容会保留，其余的没有复制。
outcome-files-summary = { $total }个项目中，已复制并检查{ $done }个。
outcome-files-memory =
    { NUMBER($count) ->
       *[other] { $count }个是通过这台笔记本电脑的内存检查的，而不是硬盘。
    }
report-show-more = 显示更多
report-showing = 显示{ $total }项中的{ $shown }项

## Component kit

dialog-close = 关闭
toast-dismiss = 知道了
code-label = 六位配对码

## Shared Quick Send words

qs-eyebrow = 快速发送
qs-eyebrow-receiving = 在这台笔记本电脑上接收
qs-route =
    .aria-label = 从{ $from }到{ $to }
qs-route-sender = 发送 · 这台笔记本电脑
qs-route-receiver = 接收 · 同一个Wi-Fi
qs-tally =
    { NUMBER($count) ->
       *[other] 已选{ $count }个项目 · { $size }
    }
qs-tally-badge = 附近发送

## Q01 Quick Send home

qs-home-title = 几个文件，直接传过去。
qs-home-body = 把几个文件发送到附近的笔记本电脑。
qs-send-title = 发送文件
qs-send-body = 在这台笔记本电脑上选择文件或文件夹。
qs-receive-title = 接收文件
qs-receive-body = 让附近的笔记本电脑把文件发送到这台电脑。
qs-history-title = 你的回执
qs-history-body = 查看接收的文件去了哪里，以及哪些没有完成。
qs-home-action = 选择文件
qs-receiving = 准备好接收了。附近的其他笔记本电脑会看到这台电脑名为{ $label }。
qs-receive-stop = 停止接收
qs-no-nearby = 附近没有找到笔记本电脑。请在另一台电脑上打开PCTwin，并让两台电脑连接同一个Wi-Fi。

## Q02 Choose files and folders

qs-choose-title = 你要发送什么？
qs-choose-body = 只选择你想分享的文件和文件夹。
qs-drop =
    .aria-label = 文件和文件夹拖放区
qs-drop-title = 把文件或文件夹拖到这里
qs-drop-body = 或者使用“添加文件”或“添加文件夹”来添加。
qs-drop-active = 松开即可添加。
qs-add-files = 添加文件
qs-add-folders = 添加文件夹
qs-remove =
    .aria-label = 移除{ $name }
qs-file-detail = { $size } · { $kind }
qs-empty = 还没有选择任何内容。请添加要发送的文件或文件夹。
qs-online-only = 有些文件仅在线存储。请先让它们在这台笔记本电脑上可用，然后再发送。
qs-protected = PCTwin无法读取{ $name }。请允许访问，或将它从列表中移除。
qs-allow = 允许访问
qs-choose-continue = 检查这些文件

## Q03 Check what you are sharing

qs-check-title = 检查你要分享的内容。
qs-check-body = 在这些文件离开这台笔记本电脑之前，请检查一遍。
qs-check-included = 包含
qs-check-shared-note = 另一台笔记本电脑能看到这些文件的名称、大小和日期。
qs-check-changed = { $name }在你选择后有变化。请再次检查。
qs-check-ack = 我已检查这些文件
qs-check-ack-hint = 发送前请移除任何私人内容。
qs-check-continue = 选择另一台笔记本电脑

## Q04 Who should receive it?

qs-who-title = 谁来接收？
qs-who-body = 选择另一台笔记本电脑上显示的名称。
qs-who-nearby = 附近 · 同一个Wi-Fi。
qs-who-code = 使用配对码
qs-who-code-body = 输入发送方显示的配对码。
qs-who-same-label = 附近有两台笔记本电脑使用这个名称。请查看另一台电脑屏幕上的名称。
qs-who-busy = { $label }正在处理另一次发送。请稍等片刻，然后再次尝试。
qs-who-note = 由另一台笔记本电脑接受文件，并选择存放的地方。
qs-who-action = 显示我的配对码

## Q08 Incoming offer (receiving laptop)

qs-offer-title = 有人想给你发送文件。
qs-offer-body = 接受之前，请检查要发送的内容。
qs-offer-note = PCTwin只保存你接受的文件，绝不会打开或运行它们。
qs-offer-accept = 接受并选择文件夹
qs-offer-decline = 拒绝这些文件
qs-offer-declined = 你已拒绝。这台笔记本电脑上没有保存任何内容。
qs-offer-changed = 发送方更改了文件。接受之前请检查新的列表。
qs-offer-gone = { $label }已不可用。这台笔记本电脑上没有保存任何内容。

## Q09 Where should the files land?

qs-land-title = 文件要放到哪里？
qs-land-body = 选择这台笔记本电脑上的文件夹。
qs-land-folder-label = 接收文件夹
qs-land-choose = 选择其他文件夹
qs-land-space-title = 可用空间
qs-land-space = 可用{ $free } · 需要{ $needed }。
qs-land-status-fits = 放得下
qs-land-status-no-space = 空间不足
qs-land-no-space = 这个文件夹有{ $free }可用空间，但这些文件需要{ $needed }。请选择其他文件夹或腾出空间。
qs-land-unavailable = 这个文件夹不可用。请连接它所在的硬盘，或选择其他文件夹。
qs-land-denied = 这个文件夹不允许PCTwin保存文件。请选择其他文件夹。
qs-land-note = 如果那里已有同名文件，在保存任何内容之前，由你选择如何处理。
qs-land-confirm = 把这些文件放到这个文件夹
qs-land-confirm-hint = 只有你接受的文件会保存在这里。
qs-land-use = 使用这个文件夹

## Q10 Keep your existing work safe

qs-conflict-title = 确保已有的文件安全。
qs-conflict-body = 选择已有同名文件时如何处理。
qs-conflict-col-file = 项目
qs-conflict-col-existing = 已有文件
qs-conflict-col-choice = 选择
qs-conflict-existing = 编辑于{ $when }
qs-conflict-none = 没有已有副本
qs-conflict-select =
    .aria-label = { $name }如何处理
qs-keep-both = 两个都保留
qs-skip = 跳过这个项目
qs-add-new = 添加新文件
qs-keep-both-note = “两个都保留”会让已有的文件保持原样。新文件将保存为{ $newName }。
qs-no-conflicts = 文件夹中还没有这些文件。不会替换任何内容。
qs-conflict-target-changed = 如果已有的文件发生变化或无法打开，它会等待你再次检查。
qs-conflict-approve = 我同意这些选择
qs-conflict-continue = 确认这些选择

## Q11 Ready to send these files?

qs-approve-title = 准备好发送这些文件了吗？
qs-approve-body = 开始之前，请仔细检查要发送的内容。
qs-approve-receiver-title = 另一台笔记本电脑
qs-approve-receiver-waiting = 正在等待另一台笔记本电脑接受并选择文件夹。
qs-approve-receiver-ready = 另一台笔记本电脑已接受，并选择了文件夹以及如何处理已有文件。
qs-approve-files-title = 文件
qs-approve-files-same = 发送完成之前，请不要更改这些文件。
qs-approve-files-changed = 有些文件在你检查后发生了变化。请再次检查。
qs-approve-existing-title = 已有文件
qs-approve-existing-body = 按照选择，两个都保留。
qs-status-waiting = 等待中
qs-status-accepted = 已同意
qs-status-unchanged = 待查看
qs-status-changed = 已更改
qs-approve-ack = 发送这些文件
qs-approve-ack-hint = 我同意这些文件和这台笔记本电脑。
qs-approve-start = 开始发送
qs-approve-cleared = 你更改了文件，所以{ $label }需要重新接受。
qs-approve-withdrawn = { $label }不再接受这次发送。没有发送任何内容。

## Q12 Send progress

qs-sending-title = 你的文件正在发送中。
qs-sending-body = 副本直接从一台笔记本电脑传到另一台。
qs-sending-progress =
    .aria-label = 正在发送选择的文件
qs-sending-current = 正在发送：{ $name }
qs-sending-speed = { $speed }（实测）
qs-step-copying = 正在复制
qs-step-copying-body =
    { NUMBER($done) ->
       *[other] 已接收{ $done }个选择的项目。
    }
qs-step-checking = 正在检查
qs-step-checking-body = 每个文件都会与原文件进行比对。
qs-step-receipt = 回执
qs-step-receipt-body = 最后可以查看文件去了哪里，以及哪些没有完成。
qs-sending-paused = 已暂停。已接收并检查的文件会保留。
qs-sending-disconnected = 与{ $label }的连接已断开。已检查的文件会保留。正在自动重新连接……
qs-sending-needs-action =
    { NUMBER($count) ->
       *[other] { $count }个项目需要你处理。
    }
qs-pause = 暂停发送
qs-resume = 恢复
qs-compact = 精简显示
qs-stop = 停止这次发送
qs-sending-complete-title = 发送已完成
qs-sending-complete-body = 每个送达的文件都与原文件一致。看看它们保存在哪里。
qs-sending-receipt = 查看回执

## Q14 Handover receipt (both laptops)

qs-receipt-title = 这是你的回执。
qs-receipt-body = 查看具体送达了什么，以及放在了哪里。
qs-receipt-box-title = 你的文件已送达
qs-receipt-box-body = 每个副本都与原文件进行了比对，并且一致。
qs-receipt-arrived =
    { NUMBER($count) ->
       *[other] 已接收{ $count }个选择的项目
    }
qs-receipt-size = { $size }与原文件一致
qs-receipt-saved-as = { $path }
qs-receipt-not-arrived = 这个文件夹没有全部送达并通过检查。
qs-receipt-status-verified = 已检查
qs-receipt-status-unfinished = 未完成
qs-receipt-not-completed = 没有完成。
qs-receipt-stopped-row = 停止前已接收并检查。
qs-receipt-partial =
    { NUMBER($count) ->
       *[other] 还有{ $count }个项目需要留意。
    }
qs-receipt-stopped = 这次发送已停止。完整送达的文件会留在你选择的文件夹中。
qs-receipt-not-opened = PCTwin没有打开或运行任何接收的文件。
qs-receipt-ack = 确认回执
qs-receipt-acked = 已于{ $when }确认回执。
qs-receipt-ack-note = 确认这份回执并不代表文件已被打开或使用。
qs-receipt-offline = { $label }处于离线状态，所以还没有看到这份回执。回执已保存在这台笔记本电脑上。
qs-receipt-missing = 发送缺失的部分
qs-receipt-missing-body = 只检查没有送达的文件。
qs-receipt-history = 所有回执
qs-receipt-done = 返回快速发送

## Q19 Finish the missing part

qs-missing-title = 完成缺失的部分。
qs-missing-body = 根据之前的回执，查看哪些没有送达。
qs-missing-status-there = 已送达
qs-missing-status-missing = 需要处理
qs-missing-there-body = 已送达，并与原文件一致。
qs-missing-missing-body = 检查缺失或没有完成的文件。
qs-missing-note = 已送达的文件会保留。新增或更改的文件需要你再次同意。
qs-missing-ack = 发送缺失的文件
qs-missing-ack-hint = PCTwin会再次检查这些文件，另一台笔记本电脑也会再次接受它们。
qs-missing-complete = 所有内容都已送达，没有需要发送的了。
qs-missing-unavailable = 找不到之前的回执，所以PCTwin无法判断缺少什么。请改为开始一次新的发送。
qs-missing-continue = 检查将要发送的内容

## Q20 Stop this send? (and decline)

qs-stop-title = 停止这次发送？
qs-stop-body = 停止之前，请检查哪些已经送达。
qs-stop-arrived-title = 完整送达的文件
qs-stop-arrived-body = 留在你选择的文件夹中。
qs-stop-rest-title = 尚未发送的文件
qs-stop-rest-body = 停止后不会再发送任何新内容。
qs-stop-partial-title = 部分送达的文件
qs-stop-partial-body = 绝不会以真实名称保存，所以不会被误认为完整的副本。
qs-stop-status-keep = 保留
qs-stop-status-stop = 停止
qs-stop-status-review = 待查看
qs-stop-ack = 停止这次发送
qs-stop-ack-hint = 完整送达的文件不会被删除。
qs-stop-confirm = 停止并查看回执
qs-stop-keep = 继续发送
qs-decline-title = 拒绝这些文件？
qs-decline-body = 这台笔记本电脑上不会保存任何内容，发送方会被告知你已拒绝。
qs-decline-confirm = 拒绝
qs-already-finished = 这次发送已经完成。请查看回执，了解送达了什么。

## F08 Your moves, kept on this laptop (history)

history-title = 你的迁移记录，保存在这台笔记本电脑上。
history-body = 再次打开报告，或继续已保存的迁移。
history-row-move = { $from } → { $to }
history-row-sent = 已发送到{ $label }
history-row-received = 已从{ $label }接收
history-status-finished = 已完成
history-status-steps-left =
    { NUMBER($count) ->
       *[other] 已完成，还有{ $count }个步骤需要你处理
    }
history-status-paused = 已暂停
history-status-stopped = 已安全停止
history-status-partial = 未全部送达
history-open-report = 报告
history-open-receipt = 回执
history-continue = 继续
history-export = 导出历史记录
history-exported = 历史记录已保存为{ $path }。
history-export-failed = 无法将历史记录保存到那里。请选择其他文件夹，然后再次尝试。
history-note = 这份历史记录只保存在这台笔记本电脑上。除非你自己导出并分享，否则不会发送到任何地方。
history-empty = 你的第一次迁移及其报告会显示在这里。
history-empty-title = 还没有迁移记录
history-start = 开始新的迁移

## Quick Send: words the mockups do not have (added while building; same voice)

qs-soon-note = 这个版本还没有开启快速发送。没有发送或接收任何内容。
qs-tally-counting =
    { NUMBER($count) ->
       *[other] 已选{ $count }个项目 · 仍在计算大小
    }
qs-gone = { $name }已不在原处。它可能已被移动或删除。请取消勾选，或者把它放回原处后重新添加。
qs-choose-why-problems = 部分已勾选的项目暂时无法发送。请取消勾选，或按说明解决后继续。
qs-check-missing = { $name }已不在原处。请返回将它移除，或者把它放回原处。
qs-check-unreadable = PCTwin无法再读取{ $name }。请允许访问，或返回将它移除。
qs-check-show-all =
    { NUMBER($count) ->
       *[other] 显示全部{ $count }个文件
    }
qs-check-change = 更改要发送的内容
qs-land-none = 还没有选择文件夹。
qs-land-space-unknown = PCTwin无法测量这里的可用空间。这些文件需要{ $needed }。
qs-conflict-existing-unknown = 已有文件
qs-conflict-choice-open = 尚未选择
qs-conflict-why = 为每个文件选择处理方式，然后勾选方框，即可继续。
qs-approve-folder-title = 接收文件夹
qs-status-recipient-choice = 接收方选择
qs-sending-card = 正在发送选择的文件
qs-progress-of = 共 { $total }
qs-receipt-partial-title = 并非所有内容都已送达
qs-receipt-metric-items =
    { NUMBER($count) ->
       *[other] 个已选项目已接收
    }
qs-receipt-metric-size = 已与源文件比对
qs-receipt-sent-as = 发送时的名称：{ $path }
qs-receipt-renamed = 那里已有同名文件，所以这个文件使用了新名称。
qs-receipt-chip-already = 已存在
qs-receipt-already = 那里已有一个相同的文件（{ $path }），所以没有再次发送。
qs-receipt-chip-declined = 未接受
qs-receipt-declined-send = 另一台笔记本电脑没有接受这个文件，所以没有发送。
qs-receipt-declined-receive = 你没有接受这个文件，所以没有保存。
qs-receipt-more = 更多详情
qs-receipt-empty = 没有发送任何内容，所以没有可显示的。
qs-receipt-already-count =
    { NUMBER($count) ->
       *[other] { $count }个项目已作为相同的文件存在
    }
qs-missing-new-send = 开始新的发送
history-open-name = { $what }：{ $title }

## R05 撤销：撤销在每个硬盘上会做什么，以及每个副本的结果

undo-here-removes = 撤销会删除 PCTwin 制作、你没有改动过的副本。你改动过的内容都会保留。
undo-here-to-trash = 撤销会把 PCTwin 制作、你没有改动过的副本移到废纸篓。你改动过的内容都会保留，也不会删除任何东西。
undo-here-keeps-fat = 撤销会保留 { $drive } 上的每个副本。这个硬盘使用 FAT 或 exFAT，它会重复使用区分文件的编号，所以 PCTwin 无法确定哪些文件是它自己的。你可以自己删除它们。
undo-here-keeps-other = 撤销会保留 { $drive } 上的每个副本。在这种硬盘上，PCTwin 无法安全地移动或删除文件。你可以自己删除它们。
undo-here-more = 更多详情
undo-here-more-trash = Mac 无法让 PCTwin 确认没有其他应用还在写入某个文件，所以 PCTwin 从不删除文件。它会把每个副本移到同一个硬盘上的废纸篓。你可以从废纸篓把它放回原处。
undo-here-more-fat = 在 FAT 和 exFAT 硬盘上，新文件可能会得到某个已删除文件用过的编号。PCTwin 只处理它能证明属于自己且未改动的文件，所以会保留这些文件。
undo-done-trashed = 已移到废纸篓，名称为“{ $name }”。你可以从废纸篓把它放回原处。
undo-kept-fat = 已保留。这个硬盘（FAT 或 exFAT）会重复使用区分文件的编号，所以 PCTwin 无法确定这是它的副本。如果你不需要它，可以自己删除。
undo-kept-no-trash = 已保留。PCTwin 无法把这个副本移到它所在硬盘上的废纸篓。如果你不需要它，可以自己删除。
