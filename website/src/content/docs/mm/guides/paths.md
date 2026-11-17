---
title: "Path ဆွဲပြီး အသုံးပြုရန်"
description: "Bézier anchor များပြင်ခြင်း၊ Selection ပြောင်းခြင်းနှင့် Path အတိုင်း Stroke ဆွဲနည်း။"
translationKey: guides/paths
sourceRevision: "path-gestures-4"
translatedRevision: "path-gestures-4"
---

## Path ဆွဲရန် {#steps}

1. **Path** (`P`) ရွေးပါ။ ထောင့် Anchor ထားရန် Click လုပ်ပါ။ မျဉ်းကွေး Handle ထုတ်ရန် Anchor ချစဉ် ဖိဆွဲပါ။
2. **Enter** နှိပ်ခြင်း သို့မဟုတ် အခြား Tool ရွေးခြင်းက Path ကို မပိတ်ဘဲ ပြင်ဆင်ခြင်းကို အဆုံးသတ်သည်။ ထို့နောက် Canvas လွတ်နေရာကို Click လုပ်လျှင် Path အသစ်စသည်။ ရှိပြီးသား Path ပြင်ရန် Node တစ်ခုကို Click လုပ်ပါ။ ဖွင့်ထားသော Path ဆက်ဆွဲရန် အစွန်းနှစ်ဖက်ထဲမှ တစ်ဖက်ကို ရွေးပြီး ထိုဘက်၌ Node များ ထပ်ထည့်ပါ။ အလယ် Node ကို ပြင်ပြီးနောက် Node ထပ်ထည့်လိုလျှင် အစွန်း Node ကို ရွေးပါ၊ သို့မဟုတ် Enter နှိပ်ပြီး Path အသစ်စပါ။
3. Node ၏ လေးထောင့်ကွက်ကို တစ်ခါတည်း ဖိဆွဲပြီး ရွှေ့နိုင်သည်။ ရွေးထားသော Node များ၏ Handle ကိုသာ ပြသည်။ Curve ကွေးရန် ထို Handle ကို ဆွဲပါ။ အောက်ပါ ပြင်ဆင်မှုများအတွက် macOS တွင် **Cmd**၊ Windows/Linux တွင် **Ctrl** သုံးပါ။ Node ကို မရွှေ့ဘဲ ညီမျှသော Handle နှစ်ဖက်ထုတ်ရန် **Cmd/Ctrl-drag** လုပ်ပါ။ Node အနည်းဆုံး သုံးခုရှိသော Outline ကို ပိတ်ရန် **ပထမ Node ကို Cmd/Ctrl-click** လုပ်ပါ။ Escape က Path ကို မပိတ်ဘဲ ပြင်ဆင်ခြင်းကို အဆုံးသတ်ပြီး မပြီးသေးသော Drag ကို ပယ်ဖျက်သည်။ မြင်ရသော Curve ကို **Cmd/Ctrl-click** (သို့မဟုတ် **Alt/Option-click**) လုပ်လျှင် ပုံသဏ္ဌာန်မပြောင်းဘဲ Node အသစ်ထည့်သည်။ ထိုမှတ်ကို ဆက်ဆွဲပြီး နေရာပြောင်းနိုင်သည်။
4. Anchor များကို Shift-click ဖြင့် အစုလိုက်ရွေးပါ။ ရွေးထားသော Anchor တစ်ခုကို ဆွဲလျှင် အစုလိုက်ရွှေ့သည်။ **Corner**၊ **Smooth** နှင့် Delete/Backspace လည်း ရွေးထားသောအစုကို သက်ရောက်သည်။ Tool Options တွင် **All Anchors** နှင့် **Deselect** ရှိသည်။
5. Layers ဘေးရှိ **Paths** တွင် Path ရွေးပြီး Visibility ပြောင်းနိုင်သည်။ ဖျောက်ထားသော Path ကို ရွေးထားသော်လည်း Canvas လွတ်နေရာကို Click လုပ်လျှင် မြင်ရသော Path အသစ်စသည်။ ဖျောက်ထားသော Path ၏ ပုံသဏ္ဌာန်ကို မပြောင်းပါ။

## Selection ပြောင်းရန် သို့မဟုတ် Stroke ဆွဲရန် {#use}

**Select → Visible Paths to Selection** က မြင်ရသော ပိတ်ပြီးသား Path များကို Selection အဖြစ် ပေါင်းပေးသည်။ ဖွင့်ထားသော Path ဖြင့် Selection ဧရိယာ မဖွဲ့နိုင်ပါ။ **Select → Selection to Path** က လက်ရှိ Selection မှ ပိတ်သော Path များ ဖန်တီးသည်။

Path အတိုင်း Stroke ဆွဲရန် မြင်ရသော Path နှင့် ပုံဆွဲနိုင်သော မြင်ရသည့် Raster Layer ကို ရွေးပါ။ **Edit → Stroke Path…** တွင် Pencil/Brush၊ Size၊ Opacity၊ Hardness၊ Flow နှင့် Buildup ပြင်ပါ။ Foreground Color ဖြင့် ဆွဲရန် **Stroke** နှိပ်ပါ။ **Paths** panel ၏ **Stroke** ခလုတ်က Dialog မဖွင့်ဘဲ Shared Settings ဖြင့် ချက်ချင်းဆွဲပေးသည်။

## ရလဒ် {#result}

Project က Editable Path ကို Stroke ဆွဲထားသော Pixel များနှင့် သီးခြားသိမ်းထားသည်။ နောက်မှ Path ပြင်လည်း ယခင် Stroke ကို ပြန်မဆွဲပါ။ Convert သို့မဟုတ် Stroke မလုပ်မချင်း Path က လမ်းညွှန်အဖြစ်သာ ရှိသည်။ Path ကို ပြထားရုံဖြင့် Export ပုံထဲ မပါပါ။

## Handle ဆွဲပုံ {#handles}

**Independent** က Handle တစ်ဖက်ကိုသာ ရွှေ့သည်။ **Aligned** က ဆန့်ကျင်ဘက် Handle ကို တစ်မျဉ်းတည်းထားပြီး အရှည်ကို မပြောင်းပါ။ **Symmetric** က ဆန့်ကျင်ဘက် Handle ကို Direction နှင့် Length ညီအောင် လုပ်ပေးသည်။ ဤ Settings များက နောက် Handle ဆွဲမှုများကို သက်ရောက်သည်။ **Corner** က ရွေးထားသော Handle များကို ခေါက်သိမ်းသည်။ **Smooth** က Handle များကို ညှိပေးသည်၊ မရှိလျှင် ဖန်တီးပေးသည်။

Path အတိုင်း Text ထားရန် Hard line break များကို အရင်ဖယ်ပါ။ Fixed box အတွင်း ဝင်နေသော်လည်း စာကြောင်းများစွာပါလျှင် လက်မခံပါ။ မြင်ရသော Editable Text နှင့် Path ရွေးပြီး **Layer → Text Along Path…** သုံးပါ။ တစ်ကြောင်းစာကို Alignment၊ Offset နှင့် Reverse Direction ဖြင့် Preview ကြည့်နိုင်သည်။ Apply လုပ်လျှင် Raster Layer အသစ်ထည့်ပြီး မူရင်း Text ကို ဖျောက်သည်။ Source နှင့် Path က Project ထဲတွင် ကျန်သည်။

## ဆက်ဖတ်ရန် {#related}

- [Selection၊ Antialias နှင့် Feather](../../basics/selections/)
- [Text၊ Drop Shadow နှင့် Outline ထည့်ရန်](../../guides/text-effects/)
- [ပုံဆွဲရန် Tools](../../reference/painting/)
