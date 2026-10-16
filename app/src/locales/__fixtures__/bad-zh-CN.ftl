# Chinese keeps only the other category; a one variant and a wrong default are planted.

files-found =
    { NUMBER($count) ->
        [0] 没有找到文件。
        [one] 找到 { $count } 个文件。
       *[other] 找到 { $count } 个文件。
    }

# plural: the only variant forgets the count
items-left =
    { $count ->
       *[other] 还有一些物品。
    }
