---
title: "Layers နှင့် Transparency"
description: "Layer ဖန်တီးခြင်း၊ စီခြင်း၊ မြင်ရပုံပြင်ခြင်း၊ Lock နှင့် Movement Link သုံးနည်း။"
translationKey: "basics/layers"
sourceRevision: "1.2.3-guide-2"
translatedRevision: "1.2.3-guide-2"
---

## ပြင်မည့် Layer ရွေးရန် {#active-layer}

Document ထဲတွင် Layers ကို အပေါ်အောက် စီထားသည်။ အပေါ် Layer ၏ Transparent နေရာမှ အောက် Layers ကို မြင်ရသည်။ ပေါင်းစပ်ပုံတွင် Transparent ဖြစ်နေသော နေရာကို Checkerboard ဖြင့် ပြသည်။

**Layers** ထဲရှိ Row ကို နှိပ်ပြီး Active လုပ်ပါ။ ပုံဆွဲခြင်း၊ Fill နှင့် Effect အများစုက ထို Layer ကိုသာ သက်ရောက်သည်။ မြင်ရသော Layers အားလုံးကို ပြင်ခြင်း မဟုတ်ပါ။ Document တစ်ခုတည်းထဲ အခြားပုံထည့်ရန် **File → Open as Layers…** ကို သုံးပါ။

## Layer ဖန်တီးပြီး စီရန် {#organize}

- **ဖန်တီးရန်:** **Layer → New Layer…** ရွေးပြီး မှတ်မိလွယ်သော အမည်ပေးပါ။ Fill ရွေးပြီး **Create** နှိပ်ပါ။ **Transparent** က ပုံဆွဲရန် Canvas အရွယ်ရှိ Layer အလွတ် ဖန်တီးသည်။ **Foreground** သို့မဟုတ် **Background** က ထို Swatch ဖြင့် ဖြည့်သည်။ Layer အသစ်က Active Layer အပေါ်တွင် ရှိမည်။
- **အမည်ပြောင်းရန်:** Row ရွေးပြီး **Layer → Rename Layer…** သုံးပါ၊ သို့မဟုတ် Right-click လုပ်ပြီး **Rename Layer…** ရွေးပါ။ Background၊ Photo၊ Caption ကဲ့သို့ အမည်များက Layer ရှာရလွယ်စေသည်။
- **မိတ္တူလုပ်ရန်:** မစမ်းပြင်မီ **Layer → Duplicate Layer** သုံးပါ။ Editable Text မိတ္တူတွင်လည်း Text ကို ဆက်ပြင်နိုင်သည်။ မိတ္တူက Movement Link နှင့် Source Attachment မပါဘဲ စသည်။
- **စီရန်:** Row ကို အပေါ်အောက် ဆွဲပါ၊ သို့မဟုတ် **Layer → Raise Layer / Lower Layer** သုံးပါ။ ပုံကို မဖုံးစေရန် Opaque နောက်ခံကို အောက်တွင် ထားပါ။
- **ဖျက်ရန်:** ရည်ရွယ်သော Row ရွေးပြီး **Layer → Delete Layer** သို့မဟုတ် Layer Context menu ၏ **Delete Layer** ကို သုံးပါ။ မှားဖျက်လျှင် **Edit → Undo** ဖြင့် ပြန်ယူပါ။ Document တွင် အနည်းဆုံး Layer တစ်ခု ကျန်ရမည်။ Canvas ပေါ်ရှိ Delete/Backspace က Layer ဖျက်ခြင်းမဟုတ်ဘဲ Pixel များကို ရှင်းသည်။

## ပုံမြင်ရပုံ ပြင်ရန် {#appearance}

1. Row ရှိ Visibility ကို ပိတ်ဖွင့်ပြီး ထို Layer ပါ/မပါ နှိုင်းယှဉ်ပါ။ Crop၊ Transform သို့မဟုတ် Gradient preview ရှိလျှင် အရင် Apply သို့မဟုတ် Cancel လုပ်ပါ။
2. Layer တစ်ခုလုံး၏ Opacity လျှော့ရန် **Layer → Layer Opacity…** ကို သုံးပါ။
3. အပေါ်အောက် အစီအစဉ်ကို **Layer → Raise Layer** သို့မဟုတ် **Lower Layer** ဖြင့် ပြောင်းပါ။ အပေါ် Layer ၏ Opaque pixels က အောက် Layer ကို ဖုံးသည်။
4. ထပ်နေသော Colors ရောစပ်ပုံပြောင်းရန် Layers panel မှ Blend mode ကို ရွေးပါ။ သာမန် Layer ထပ်ပုံအတွက် **Normal** မှ စပါ။ အဖြူပေါ် အမည်းမျဉ်းများပါသော Layer ကို ရောင်စုံ Layer အပေါ်တင်ပြီး **Multiply** စမ်းကြည့်ပါ။ အဖြူက အောက် Color ကို မဖုံးတော့ဘဲ အမည်းမျဉ်းများ ကျန်သည်။ အနက်ပေါ် အလင်းစက်များပါသော Texture ကို ဓာတ်ပုံအပေါ်တင်ပြီး **Screen** စမ်းပါ။ အနက်က ဓာတ်ပုံကို မဖုံးတော့ဘဲ အလင်းနေရာများက ပိုလင်းစေသည်။ ရလဒ်ပြင်းလွန်းလျှင် Layer Opacity လျှော့ပါ။ သာမန်ထပ်ပုံ ပြန်လိုလျှင် Normal ပြန်ရွေးပါ။

**ရလဒ်:** Canvas ပေါ်ရှိ ပေါင်းစပ်ပုံ ပြောင်းသော်လည်း Layer တစ်ခုစီ သီးခြား ကျန်မည်။ Layers ကို ထိန်းထားရန် `.lime` ဖြင့် Save လုပ်ပါ။ Export က Layers ပေါင်းထားသော ပုံတစ်ပုံ ထုတ်ပေးသည်။

## Layer တစ်ခုတည်း လိုမှ Merge လုပ်ရန် {#merge}

ကပ်လျက် Raster Layer နှစ်ခုထဲမှ အပေါ် Layer ရွေးပြီး **Layer → Merge Down** သုံးပါ။ Opacity၊ Mask နှင့် Effects ကို Pixel များထဲ Bake လုပ်ကာ Raster Layer တစ်ခုတည်း ဖြစ်သွားမည်။ Undo သို့မဟုတ် ယခင် Project မိတ္တူ မရှိလျှင် နှစ်ပိုင်းကို သီးခြားရွှေ့ခြင်း၊ ပြန်ပြင်ခြင်း မရတော့ပါ။ ရလဒ်တွင် ထို Layers ၏ Movement Link နှင့် Attachment များလည်း မကျန်ပါ။

Layer နှစ်ခုလုံး မြင်ရပြီး Rasterize လုပ်ထားကာ Alpha/Position Lock မရှိရပါ။ Blend mode သို့မဟုတ် ပုံအရွယ်ကန့်သတ်ချက်ကြောင့်လည်း Merge Down မရနိုင်ပါ။ **Normal** က စသုံးရန် အလွယ်ဆုံးဖြစ်သည်။ သီးခြားဆက်ပြင်လိုသေးလျှင် Layers ကို မပေါင်းပါနှင့်။ Export က မြင်ရသောပုံကို သီးခြားပုံတစ်ပုံအဖြစ် ပေါင်းထုတ်ပြီးသားဖြစ်သောကြောင့် မျှဝေရန် Merge လုပ်ရန် မလိုပါ။

## Lock နှင့် အတူရွှေ့ခြင်း {#locks-links}

- **Alpha Lock** က Raster layer ၏ ရှိပြီးသား Transparency ကို ထိန်းသည်။ ရှိပြီးသား Pixel များ၏ Color ကို ပြောင်းနိုင်သော်လည်း Eraser ဖြင့် မဖယ်နိုင်ပါ။ Editable Text တွင် သုံးမရပါ။
- Row တစ်ခုစီရှိ Padlock သည် **Position Lock** ဖြစ်သည်။ Layer ကို ကိုယ်တိုင်ရွှေ့ခြင်းနှင့် Transform လုပ်ခြင်းကို ပိတ်သည်။ Image တစ်ခုလုံး ပြောင်းသော Operations က Lock ရှိသော်လည်း သက်ရောက်သည်။
- Chain ခလုတ်ဖြစ်သော **Movement Link** ဖွင့်ထားသည့် Layers က အတူရွှေ့သည်။ တစ်ခုရွှေ့လျှင် အခြားဖွင့်ထားသော Layers၊ Hidden layers ပါ လိုက်သည်။ ပါဝင်သော Layer တစ်ခု Position Lock ရှိလျှင် အားလုံး ရွှေ့မရပါ။

Shadow နှင့် Outline layer အသစ်များက Source ကို လိုက်ရွှေ့ရန် ချိတ်ထားသည်။ Source ရွှေ့လျှင် ထို Layers ပါ လိုက်သည်။ Shadow ကို တိုက်ရိုက်ရွှေ့လျှင် Source နှင့် အကွာအဝေးကို ပြောင်းသည်။ သီးခြားဖြစ်စေရန် **Layer → Detach from Source** ကို သုံးပါ။ နောက်မှ Source ပြင်လည်း Shadow pixels ကို အလိုအလျောက် ပြန်မဖန်တီးပါ။

## ဆက်ဖတ်ရန် {#related}

- [ပုံများ ပေါင်းပြီး နေရာချရန်](../../guides/combine-images/)
- [Layer mask သုံးရန်](../../basics/masks/)
- [Layer ရွှေ့မရ၊ Transform မရလျှင်](../../troubleshooting/moving/)
