---
title: "Text၊ Export နှင့် Recovery ပြဿနာများ"
description: "Font မရှိခြင်း၊ Flattened Export၊ PDF ကန့်သတ်ချက်နှင့် Session ပြတ်တောက်ခြင်းကို ဖြေရှင်းနည်း။"
translationKey: troubleshooting/files-text-recovery
sourceRevision: "text-styles-1"
translatedRevision: "text-styles-1"
---

## Text ပုံစံကွာခြားခြင်း သို့မဟုတ် ဖြတ်နေခြင်း {#text}

သတ်မှတ်ထားသော Font မရှိလျှင် အစားထိုးသုံးသည်။ Lime ၏ Notice ကို စစ်ပြီး ရှိသော Font ရွေးပါ။ မူရင်း Font သုံးရန် Project Save လုပ်ပြီး Font ထည့်ပါ။ Lime ကို ပိတ်၍ ပြန်ဖွင့်ပါ။ Project ပြန်ဖွင့်ပြီးမှ Text ပြင်ပါ။ App က လက်ရှိ Session အတွက် Font list ကို Cache လုပ်ထားသည်။ Text controls အပေါ်ရှိ **All text**၊ **Selected text** သို့မဟုတ် **New typing** က မည်သည့်စာသားကို ပြောင်းမည်ကို ပြသည်။ စာသားအပိုင်းကို Highlight လုပ်ပြီး ၎င်း၏ Font၊ Size၊ Color၊ Bold သို့မဟုတ် Italic ကို သီးခြားပြောင်းနိုင်သည်။ **Mixed** ဆိုသည်မှာ ရွေးထားသောစာသားတွင် တန်ဖိုးမတူညီခြင်း ဖြစ်သည်။ တစ်ခုကိုပြောင်းလျှင် ကျန် Style များ မပြောင်းပါ။

**Fixed box** ဖွင့်လျှင် Width/Height ပြင်နိုင်ပြီး ထိုအရွယ်အတွင်း စာခေါက်သည်။ Box အပြင်ရောက်သော Text ကို ဖြတ်ထားသည်။ Overflow ပြလျှင် Width/Height တိုးပါ၊ သို့မဟုတ် ကန့်သတ်ချက်ဖယ်ရန် Fixed box ပိတ်ပါ။ Source Text က ကျန်နေသည်။ Native input-method composition မရသဖြင့် အခြား App တွင် စာရေးပြီး Lime ထဲ Paste လုပ်ပါ။

Escape က Text draft ကို Commit လုပ်သည်။ စာရိုက်မှု ပြန်ယူရန် Undo သုံးပါ။ Escape နှိပ်လျှင် စာရိုက်ထားသည်များကို ပယ်ဖျက်မည် မဟုတ်ပါ။ Pixel Tools သုံးရန် Rasterize လုပ်ရပြီး Editable source ဖယ်သွားသည်။ `.lime` Copy အရင် Save လုပ်ပါ။

## Export တွင် Layer သို့မဟုတ် Transparency မပါခြင်း {#export}

**File → Export…** က အမြဲ Flattened copy ရေးသည်။ Layer၊ Path နှင့် Text ဆက်ပြင်ရန် `.lime` Project ပြန်ဖွင့်ပါ။ Export က Project ကို Saved အဖြစ် မသတ်မှတ်သဖြင့် Project ကို သီးခြား Save လုပ်ပါ။

Transparency အတွက် PNG သို့မဟုတ် WebP သုံးပါ။ JPEG တွင် Alpha မပါပါ။ Lime ၏ PDF Export က Canvas ကို အဖြူနောက်ခံပေါ် Flatten လုပ်သည်။ Editor ရှိ Checkerboard က Transparent Pixel ကို ပြသည်။ Export ပုံထဲတွင် ပါသော Pattern မဟုတ်ပါ။

မပြီးသေးသော Crop/Gradient/Transform Draft ကို Save/Export မတိုင်မီ Apply သို့မဟုတ် Cancel လုပ်ရသည်။ Numeric နှင့် Text Edit များကို အရင်အဆုံးသတ်ပေးသည်။ Text Commit တွင် Error ဖြစ်လျှင် Draft ကျန်ပြီး Save/Export ဆက်မလုပ်ပါ။ ပြထားသောပြဿနာကို ဖြေရှင်းပြီး ပြန်စမ်းပါ။

## PDF Import မရခြင်း သို့မဟုတ် မူရင်းနှင့်ကွာခြားခြင်း {#pdf}

Encrypted PDF များကို မပံ့ပိုးပါ။ Import က ရွေးထားသောစာမျက်နှာကို DPI အလိုက် Rasterize လုပ်သည်။ ထို့ကြောင့် ထည့်ထားသောစာသားကို Text အဖြစ် မပြင်နိုင်ပါ။ First-page Canvas သုံးလျှင် ပိုကြီးသောစာမျက်နှာများ ဖြတ်နိုင်သည်။ Shared Canvas အတွက် **Fit All Pages** ရွေးပါ။ စာမျက်နှာအလိုက် အရွယ်အစားထိန်းရန် Separate Tabs သုံးပါ။ Open as Layers က ရှိပြီးသား Canvas ကို Resize မလုပ်ပါ။

Size/Memory ကန့်သတ်ချက်ဖြစ်လျှင် DPI လျှော့ပါ၊ သို့မဟုတ် စာမျက်နှာနည်းနည်းရွေးပါ။ PDF Export က Flattened page တစ်ခုသာ ဖန်တီးသည်။ Multipage သို့မဟုတ် Searchable Text PDF မဟုတ်ပါ။ DPI က Canvas ကို Resample မလုပ်ဘဲ Physical page size ပြောင်းသည်။

## ပြတ်တောက်ခဲ့သောအလုပ် ပြန်ယူရန် {#recovery}

Session ပြတ်တောက်ပြီးနောက် Lime က သီးခြား Recovery Project များကို ပြန်ဖွင့်ပြီး Notice ပြနိုင်သည်။ Recovered Tab တစ်ခုစီ စစ်ပါ။ ထို့နောက် **Save As…** ဖြင့် သိမ်းမည့်နေရာ ရွေးပါ။ Recovery က မူရင်း File ကို မရေးထပ်သကဲ့သို့ Saved revision ကိုလည်း မပြောင်းပါ။

Recovery ကို အချိန်ခြားလုပ်သဖြင့် နောက်ဆုံးစက္ကန့်အနည်းငယ် မပါနိုင်ပါ။ ဖြစ်နိုင်လျှင် Text Draft ပါသော်လည်း အခြားမအတည်ပြုရသေးသော Preview များ မပါပါ။ Recovery Error ပြလျှင် Recovered Tab မပိတ်ဘဲ Copy အသစ် Save လုပ်ပါ။ ပုံမှန် Project Save အစား Recovery ကို မမှီခိုပါနှင့်။

Automatic Recovery က Copy တစ်ခုကို မဖွင့်နိုင်လျှင် Lime က File ကို ထိန်းသိမ်းပြီး Path ကို ပြသည်။ ထို Copy ကို ကိုယ်တိုင်ပြန်ဖွင့်ရန် **File → Open…** သုံးပါ။ နောက်တစ်ကြိမ် App ဖွင့်ချိန်တွင် ထိန်းသိမ်းထားသော Copy များကို ပြန်စမ်းသည်။ မပြောင်းလဲသော Copy တွင် တူညီသော Error ဖြစ်လျှင် App ပြန်ဖွင့်တိုင်း မပြဘဲ တစ်ကြိမ်သာ သတိပေးသည်။ Copy ပြောင်းထားခြင်း သို့မဟုတ် Error မတူခြင်းဖြစ်လျှင် Warning အသစ် ပြနိုင်သည်။

## ဆက်ဖတ်ရန် {#related}

- [Project Save နှင့် ပုံ Export](../../reference/files/)
- [PDF Import နှင့် Export](../../image/pdf/)
- [Text၊ Drop Shadow နှင့် Outline ထည့်ရန်](../../guides/text-effects/)
