---
title: "Text၊ Drop Shadow နှင့် Outline ထည့်ရန်"
description: "Text ကို ဆက်ပြင်နိုင်အောင် ထားပြီး သီးခြား Drop Shadow သို့မဟုတ် Outline ထည့်နည်း။"
translationKey: guides/text-effects
sourceRevision: "text-styles-2"
translatedRevision: "text-styles-2"
---

## Editable Text ထည့်ရန် {#steps}

1. **Text** (`T`) ရွေးပြီး Canvas ၏ လွတ်နေသောနေရာတွင် Click လုပ်ပါ။ မည်သည့် Layer ကို ရွေးထားသည်ဖြစ်စေ Text အသစ်က Layer အားလုံး၏ အပေါ်ဆုံးတွင် ပေါ်သည်။ ရှိပြီးသား Text ကို Click လုပ်လျှင် ၎င်း၏ လက်ရှိ Layer အစီအစဉ်အတိုင်း ပြင်သည်။
2. စာရိုက်ပါ သို့မဟုတ် Paste လုပ်ပါ။ Return က စာကြောင်းအသစ်ထည့်သည်။ Native composition လိုအပ်သော Input method အတွက် အခြား App တွင် စာရေးပြီး Lime ထဲ Paste လုပ်ပါ။
3. **Tool Options** တွင် ထည့်သွင်းထားသော Font၊ Size၊ Color၊ Bold/Italic၊ Spacing နှင့် Alignment ရွေးပါ။ Font၊ Size၊ Color၊ Bold နှင့် Italic က Highlight လုပ်ထားသော စာသားကိုသာ ပြောင်းသည်။ စာသား Highlight မလုပ်ထားလျှင် စာရိုက်နေစဉ်မှာပင် လက်ရှိ Text Layer တစ်ခုလုံးကို ချက်ချင်းပြောင်းသည်။ စကားလုံးရွေးရန် Double-click လုပ်ပါ။ မြင်နေရသော စာတစ်ကြောင်းရွေးရန် Triple-click လုပ်ပါ။ စာခေါက်ထားလျှင် မြင်ရသော တစ်ကြောင်းချင်းစီကိုသာ ရွေးသည်။ Spacing နှင့် Alignment က Layer တစ်ခုလုံးကို အမြဲသက်ရောက်သည်။
4. Box အတွင်း စာခေါက်ရန် **Fixed box** ဖွင့်ပါ။ ထို့နောက် Width/Height ပြင်နိုင်သည်။ Overflow notice ပေါ်လျှင် စာအချို့ Box အပြင်ရောက်နေသည်။ ပိုမြင်ရန် Box ချဲ့ပါ၊ သို့မဟုတ် ကန့်သတ်ချက်ဖယ်ရန် Fixed box ပိတ်ပါ။
5. အပြင်ကို Click လုပ်ပါ၊ သို့မဟုတ် Cmd/Ctrl+Enter နှိပ်ပါ။ Escape လည်း Text draft ကို Commit လုပ်ပြီး စာရိုက်မှုကို မပယ်ဖျက်ပါ။ ပြန်ယူရန် **Edit → Undo** သုံးပါ။

Text Size ကို **ပုံ၏ Pixel 1–4,096** အထိ သတ်မှတ်နိုင်သည်။ Resolution မြင့်သောပုံများတွင် 300 သို့မဟုတ် 1,024 ကဲ့သို့သော Size များကို သုံးနိုင်သည်။ Tool Options တွင် Size ကို တိုက်ရိုက်ရိုက်ထည့်ပါ။ Text အလွန်ကြီးလျှင် Lime ၏ Layer အရွယ်အစားနှင့် Memory ကန့်သတ်ချက်အတွင်း ရှိရန် လိုသည်။

Lime ၏ Text Layer များအကြား Copy/Paste လုပ်လျှင် ရွေးထားသော Style များကို ထိန်းထားသည်။ အခြား App မှ Plain text ကို Paste လုပ်လျှင် ထည့်မည့်နေရာ၏ Style ကို သုံးသည်။ Style အားလုံး ဆက်ပြင်နိုင်ရန် `.lime` အဖြစ် Save လုပ်ပါ။ Size field ကို အလွတ်ထားခြင်း သို့မဟုတ် ၎င်းကို ပြင်နေစဉ် Escape နှိပ်ခြင်းက မူလ Size များကို ပြန်ထားသည်။

## Drop Shadow သို့မဟုတ် Outline ထည့်ရန် {#effects}

မြင်ရသော Text Layer ကို ရွေးပြီး **Filters → Drop Shadow…** သို့မဟုတ် **Filters → Stroke…** သုံးပါ။ Preview ပြင်ပြီး **Before** / **After** နှိုင်းယှဉ်ပါ။ Color ရွေးပါ၊ သို့မဟုတ် **Use Foreground** သုံးပါ။ **Reset** က မူလ Parameter နှင့် အမည်းရောင်ကို ပြန်ထားသည်။ **Apply** က အတည်ပြုပြီး **Cancel** က Document မပြောင်းဘဲ ပိတ်သည်။

Stroke က စာလုံးအတွင်းရှိ အပေါက်များအပါအဝင် အပြင်ဘက်တွင်သာ Outline လုပ်သည်။ Width သည် ပုံ၏ Pixel 0–128 အတွင်းဖြစ်သည်။ Width သို့မဟုတ် Opacity သုညဆို Layer မထည့်ပါ။ Drop Shadow နှင့် Stroke နှစ်ခုလုံးက Source အောက်တွင် Raster Layer အသစ်ထည့်ပြီး မူရင်း Text ကို Active နှင့် Editable အဖြစ် ထားသည်။

## ရလဒ် {#result}

Project ထဲတွင် Editable Text နှင့် သီးခြား Effect Layer ရှိမည်။ Source ရွှေ့လျှင် တွဲထားသော Effect လည်း ရွှေ့မည်။ နောက်မှ Text ပြောင်းလျှင် Effect ပုံကို အလိုအလျောက် ပြန်မလုပ်ပါ။ ပုံသဏ္ဌာန်ပြောင်းလျှင် Effect အဟောင်း၏ Layer ကို ဖျောက်ပါ သို့မဟုတ် ဖျက်ပါ။ ပြီးမှ အသစ်ဖန်တီးပါ။ Detach လုပ်ခြင်းက Movement ချိတ်ဆက်မှုကိုသာ ဖယ်သည်။ Pixel အဟောင်းများ ဆက်ပေါ်နေမည်။

## ပြဿနာအဖြေရှာရန် {#troubleshooting}

Font မရှိလျှင် Lime က အစားထိုးသုံးထားကြောင်း Notice ပြသည်။ ရှိသော Font ရွေးပါ၊ သို့မဟုတ် Project Save လုပ်ပြီး လိုသော Font ထည့်ပါ။ Lime ကို ပိတ်၍ ပြန်ဖွင့်ပြီး Project ပြန်ဖွင့်ကာ Text ပြင်ပါ။ Lime က လက်ရှိ App session အတွက် Font list ကို Cache လုပ်ထားသည်။ **Layer → Rasterize Text Layer** မလုပ်မီ `.lime` Project Save လုပ်ပါ။ Rasterize က Editable source ကို ဖယ်ပြီး Pixel Tools သုံးခွင့်ပေးသည်။

မျဉ်းကွေးအတိုင်း Text ထားရန် Hard line break များကို အရင်ဖယ်ပါ။ Text Along Path က စာတစ်ကြောင်းသာ လက်ခံသည်။ **Paths** တွင် မြင်ရသော Path ရွေးပြီး **Layer → Text Along Path…** သုံးပါ။ Raster result အသစ်လုပ်ပြီး မူရင်း Text ကို ဖျောက်သည်။ Text နှင့် Path ကို Live ချိတ်ဆက်ထားခြင်း မဟုတ်ပါ။

## ဆက်ဖတ်ရန် {#related}

- [Path ဆွဲပြီး အသုံးပြုရန်](../../guides/paths/)
- [Color Adjustment နှင့် Effects](../../reference/effects/)
- [Project Save နှင့် ပုံ Export](../../reference/files/)
