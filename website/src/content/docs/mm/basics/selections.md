---
title: "Selection၊ Antialias နှင့် Feather"
description: "Selection ဖြင့် ပြင်မည့်နေရာ ကန့်သတ်ခြင်း၊ ပေါင်းစပ်ခြင်း၊ အနားပြင်ခြင်းနှင့် Border ဆွဲနည်း။"
translationKey: "basics/selections"
sourceRevision: "1.2.3-selection-move-1"
translatedRevision: "1.2.3-selection-move-1"
---

## Selection အလုပ်လုပ်ပုံ {#meaning}

Selection က ပုံဆွဲခြင်း၊ Fill နှင့် Effect အများစုတွင် ပြင်နိုင်သော Pixel နေရာကို ကန့်သတ်သည်။ Active layer ကို ရွေးပေးခြင်း မဟုတ်ပါ။ Soft selection တွင် Pixel တစ်ခုကို တစ်စိတ်တစ်ပိုင်းသာ သက်ရောက်နိုင်သည်။ ထို့ကြောင့် Selection အနားဘက်တွင် Edit ၏ သက်ရောက်မှု တဖြည်းဖြည်း လျော့သွားသည်။

Selection outline က ဘောင်ကိုသာ ပြသည်။ အနားရှိ Partial coverage အားလုံးကို မပြနိုင်သဖြင့် ရလဒ်ကို Zoom ချဲ့ပြီး စစ်ပါ။ Selection ပြောင်းခြင်းကို Undo လုပ်နိုင်သည်။ Selection သာ ပြောင်းလျှင် သိမ်းထားပြီးသား Project ကို Unsaved ဟု မမှတ်ပါ။

## Selection ဖန်တီးပြီး ပေါင်းစပ်ရန် {#steps}

1. **Select** menu သို့မဟုတ် Toolbox မှ **Rectangle Select**၊ **Oval Select**၊ **Free Select** သို့မဟုတ် **Magic Wand** ကို ရွေးပါ။
2. Rectangle/Oval ကို ဆွဲပါ၊ Free outline ကို ပိတ်ပါ သို့မဟုတ် Wand ဖြင့် ဆက်နေသော နေရာကို နှိပ်ပါ။
3. နောက်ထပ် Selection လုပ်စဉ် **Shift** ကို ဖိထားပြီး ထပ်ပေါင်းပါ။ **Cmd/Ctrl** ဖြင့် နုတ်ပါ။ **Shift+Cmd/Ctrl** ဖြင့် နှစ်ခုထပ်နေသော နေရာကိုသာ ယူပါ။ macOS တွင် Command၊ Windows/Linux တွင် Ctrl ကို သုံးပါ။
4. Active layer ပေါ်တွင် ပုံဆွဲပါ သို့မဟုတ် Fill လုပ်ပါ။ အခြားနေရာပြင်ရန် ကန့်သတ်မှုဖယ်လိုလျှင် **Select → Select None** ကို ရွေးပါ။

စက်ဝိုင်းအတွက် Oval Select တွင် **1:1** ရွေးပါ။ Rectangle နှင့် Oval က Aspect ratio တစ်ခုတည်းကို မျှဝေသုံးသည်။ Crop က သီးခြား မှတ်ထားသည်။ Fixed ratio ရွေးလျှင် လက်ရှိ Editable shape ကိုလည်း Center မပြောင်းဘဲ ပြန်ညှိပေးသည်။

## Antialias နှင့် Feather {#soft-edges}

**Antialias edges** က Curve နှင့် စောင်းနေသော အနားကို ပါးလွှာသော Pixel fringe အတွင်း ချောမွေ့စေသည်။ **Feather new selections** က ပိုကျယ်သော Radius အတွင်း Coverage ကို ဖြန့်ပြီး Soft edges ဖန်တီးသည်။ နှစ်ခုက မတူသော Settings ဖြစ်ပြီး အတူသုံးနိုင်သည်။

ရှိပြီးသား Selection ကို **Select → Feather…** ဖြင့် Feather လုပ်ပါ။ **Grow…** က ချဲ့ပြီး **Shrink…** က ကျုံ့သည်။ Soft coverage ကို ထိန်းထားသည်။ Selection အသစ်အတွက် Feather setting ပြောင်းရုံဖြင့် လက်ရှိ Selection မပြောင်းပါ။

Shapes ပေါင်းစပ်ပြီးနောက် သို့မဟုတ် Coverage ပြင်ပြီးနောက် Selection က Mask ဖြစ်သွားနိုင်သည်။ ထိုအခါ မူလ Rectangle/Oval controls မရှိတော့နိုင်ပါ။ Selection mask က ပြင်နိုင်သော နေရာကို ကန့်သတ်သည်။ Layer mask က Layer ၏ Visibility ကို ထိန်းသည်။

ပေါင်းစပ်ထားသော Mask ကို ရွှေ့ရန် Rectangle Select သို့မဟုတ် Oval Select ရွေးပြီး ရွေးထားသောဧရိယာကို ဖိဆွဲပါ။ အပေါက်များနှင့် Soft edges ကိုပါ အတူရွှေ့ပေးသည်။ Rectangle/Oval resize controls မရှိလည်း ရွှေ့နိုင်သည်။

## Selection ပတ်လည် Border ဆွဲရန် {#stroke}

လေးထောင့်ဘောင်၊ စက်ဝိုင်း သို့မဟုတ် Free shape Border ဆွဲရန် သုံးနိုင်သည်။

1. Selection လုပ်ပါ။ ပြတ်သားသော Geometric Border အတွက် **Feather new selections** ပိတ်ထားပါ။ Curve အနား ချောမွေ့လိုလျှင် Antialias ကို သီးခြားရွေးပါ။
2. **Layer → New Layer…** ရွေးပြီး **Border** ဟု အမည်ပေးကာ **Transparent** Fill သုံးပါ။ Layer ကို မြင်ရအောင် ဖွင့်ထားပြီး Alpha Lock ပိတ်ထားပါ။ Border အတွက် Layer သီးခြား ရမည်။ Layer အသစ်လုပ်သော်လည်း Selection နေရာ မပြောင်းပါ။
3. Foreground color ရွေးပြီး **Edit → Stroke Selection…** သုံးပါ။
4. Hard stroke အတွက် **Pencil**၊ Hardness ပြင်နိုင်ရန် **Brush** ရွေးပါ။ **Size (px)** သတ်မှတ်ပြီး Opacity 100%၊ Buildup ပိတ်ထားသော အခြေအနေမှ စကာ **Stroke** နှိပ်ပါ။ Size က Border အထူကို ပုံ၏ Pixel ဖြင့် သတ်မှတ်သည်။
5. Border ကို စစ်ရန် **Select → Select None** သုံးပါ။ အခြား Size စမ်းလိုလျှင် **Edit → Undo** သုံးပါ။

**ရလဒ်:** Active Raster Layer ပေါ်တွင် Selection အနားတစ်လျှောက် Border ဆွဲပေးမည်။ Brush က Selection အတွင်းမှာသာ ကန့်သတ်ခြင်းမရှိဘဲ အနား၏ နှစ်ဖက်လုံးသို့ ဖြန့်သည်။ Stroke အပြည့်ဝင်ဆံ့ရန် Canvas အတွင်း နေရာချန်ပါ။ နောက်မှ Selection ပြောင်းရုံဖြင့် Border ကို ပြန်မဆွဲပါ။

**Edit → Stroke Selection…** က ကိုယ်ဖန်တီးသော Selection အတိုင်း Active Layer ပေါ်တွင် ဆွဲသည်။ **Filters → Stroke…** က Layer ၏ မြင်ရသော ပုံသဏ္ဌာန်ကို ယူပြီး အပြင်ဘက် Outline သာ ဖန်တီးကာ အောက်တွင် Layer အသစ် ထည့်သည်။ ဥပမာ [Text](../../guides/text-effects/) ကို Selection အရင်မလုပ်ဘဲ Outline ထည့်ရန် Filter ကို သုံးနိုင်သည်။

## ဆက်ဖတ်ရန် {#related}

- [Selection tools](../../reference/selection/)
- [နောက်ခံဖယ်ရန်](../../guides/remove-background/)
- [Layer mask သုံးရန်](../../basics/masks/)
- [ရွေးထားသော Pixel များကို အခြားပုံထဲ Copy လုပ်ရန်](../../guides/combine-images/#clipboard)
