---
title: "ပုံဆွဲပြီး ပြန်ပြင်ရန်"
description: "Brush ဖြင့် ပုံဆွဲခြင်း၊ Eraser သုံးခြင်းနှင့် Clone Stamp ဖြင့် ပုံပြင်နည်း။"
translationKey: guides/paint-retouch
sourceRevision: "1.2.3-tool-feedback-1"
translatedRevision: "1.2.3-tool-feedback-1"
---

## Brush ဖြင့် ပုံဆွဲရန် {#steps}

1. မြင်ရသော Raster Layer ကို ရွေးပါ။ ပုံဆွဲမှုကို သီးခြားထားရန် **Layer → New Layer…** ဖြင့် Layer အသစ်လုပ်ပါ။
2. Toolbox swatch မှ [Foreground color ရွေးပါ](../../basics/colors/)။ Soft edge အတွက် **Brush**၊ Hard edge အတွက် **Pencil** သုံးပါ။
3. Size၊ Opacity နှင့် ပါဝင်လျှင် Hardness ကို ပြင်ပါ။ `[` နှင့် `]` က Size ပြောင်းပေးသည်။ Brush ဆွဲရာတွင် ပိုငြိမ်စေရန် **Smoothing** တွင် **Light** သို့မဟုတ် **Strong** ရွေးပါ။
4. ဖိဆွဲပြီး ပုံဆွဲပါ။ Pointer လွှတ်လျှင် Stroke တစ်ခုကို Apply လုပ်သည်။ မလွှတ်မီ Escape နှိပ်လျှင် ထို Stroke ကို Cancel လုပ်သည်။ Shift ဖိထားလျှင် ယခင် Stroke အဆုံးမှ မျဉ်းဖြောင့် Guide ကို ပြသည်။ နှိပ်လိုက်လျှင် ထိုမျဉ်းကို ဆွဲပေးသည်။ Guide က Zoom/Pan အတိုင်း လိုက်ပြောင်းပြီး Pixel များကို မပြင်ပါ။
5. Pixel များကို Transparent ဖြစ်အောင် ဖျက်ရန် **Eraser** (`E`) သုံးပါ။ နေရာကွက်ပြီး Blur/Sharpen လုပ်ရန် **Tools** ရှိ **Blur Brush** / **Sharpen Brush** သုံးပါ။

## ရလဒ် {#result}

Active Layer ပေါ်ရှိ Canvas နှင့် လက်ရှိ Selection အတွင်းမှ Pixel များသာ ပြောင်းမည်။ Stroke တစ်ခုကို Undo တစ်ဆင့်ဖြင့် ပြန်ယူနိုင်သည်။ Tool တစ်ခုစီ၏ Brush Settings ကို သီးခြားမှတ်ထားပြီး App ပြန်ဖွင့်လည်း ကျန်သည်။

## Clone Stamp ဖြင့် ပုံပြင်ရန် {#clone}

**Clone Stamp** (`Shift+C`) ရွေးပါ။ မြင်ရသော Raster Layer ၏ Canvas အတွင်းရှိ Source နေရာကို Alt-click လုပ်ပါ။ ထို့နောက် ပြင်လိုသောနေရာတွင် ပုံဆွဲပါ။ မြင်ရသမျှ Layer များ၏ ပေါင်းစပ်ပုံမှ Source ယူရန် **Sample All Layers** ဖွင့်ပါ။ ပုံဆွဲမှုက Active Raster Layer ပေါ်သာ သက်ရောက်သည်။

**Aligned** က Stroke များကြား Source နှင့် Destination ၏ Offset ကို ထိန်းသည်။ ပိတ်ထားလျှင် Stroke တိုင်း ရွေးထားသော Source မှ ပြန်စသည်။ **Show Source** က နမူနာယူနေသောနေရာကို ပြသည်။ Tab/Layer ပြောင်းပြီးလျှင်၊ Layer/Canvas အရွယ်အစား သို့မဟုတ် နေရာပြောင်းပြီးလျှင် Source ပြန်ရွေးပါ။ Sampling ပြောင်းလျှင်လည်း Source ရှင်းသွားသည်။

## Stroke တစ်ခု၏ အားကို ထိန်းရန် {#coverage}

Opacity က Stroke စချိန် Pixel အခြေအနေမှ ဘယ်လောက်ပြောင်းမည်ကို ကန့်သတ်သည်။ Stroke တစ်ခုတည်းတွင် ထပ်ဆွဲလည်း ထိုပမာဏထက် မကျော်ပါ။ **Buildup** ဖွင့်ထားလျှင် **Flow** က Dab တစ်ခုစီအလိုက် Stroke ၏ Opacity limit အထိ တဖြည်းဖြည်းတိုးစေသည်။ ထပ်တိုးလိုလျှင် Stroke အသစ်စပါ။ Blur/Sharpen တွင် ထိုပမာဏကို **Strength** ဟု ခေါ်သည်။

ပုံဆွဲသော်လည်း ဘာမှမပြောင်းလျှင် Selection၊ Layer Visibility၊ Alpha Lock နှင့် Opacity/Flow သုညဖြစ်မဖြစ် စစ်ပါ။ Alpha Lock က ရှိပြီးသား Alpha ပေါ်တွင်သာ Color ပြောင်းခွင့်ပေးပြီး Eraser ဖြင့် ဖျက်ခြင်းကို တားသည်။

## ဆက်ဖတ်ရန် {#related}

- [ပုံဆွဲရန် Tools](../../reference/painting/)
- [ပုံဆွဲခြင်း သို့မဟုတ် Fill လုပ်သော်လည်း မပြောင်းခြင်း](../../troubleshooting/painting/)
- [Layers နှင့် Transparency](../../basics/layers/)
