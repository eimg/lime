---
title: "Selection Tools"
description: "Rectangle၊ Oval၊ Free Select နှင့် Magic Wand တို့၏ Settings နှင့် အသုံးပြုပုံ။"
translationKey: reference/selection
sourceRevision: "1.2.3-selection-move-1"
translatedRevision: "1.2.3-selection-move-1"
---

Selection က ပုံဆွဲခြင်း၊ Fill၊ Clear နှင့် Effects အများအပြားကို ကန့်သတ်ပေးသည်။ Selection က Active Layer မဟုတ်ဘဲ Canvas နှင့် သက်ဆိုင်သည်။ Layer ပြောင်းလည်း Selection နေရာမပြောင်းပါ။

Active မြင်ရသော Raster Layer ၏ ရွေးထားသောနေရာကို Transparent ဖြစ်အောင် **Delete** သို့မဟုတ် **Backspace** နှိပ်ပါ၊ သို့မဟုတ် **Edit → Clear to Transparency** သုံးပါ။ Soft edge များကို Coverage အချိုးအတိုင်း ဖျက်သည်။ Selection ကန့်သတ်ချက်မရှိလျှင် Layer တစ်ခုလုံးကို ဖျက်သည်။ **Undo** ဖြင့် ပြန်ယူနိုင်သည်။ Raster Layer အားလုံးတွင် Transparency သုံးနိုင်သည်။ Alpha Lock က Clear ကို တားသည်။ Editable Text ၏ Pixel များကို ဖျက်ရန် အရင် Rasterize လုပ်ပါ။

## Tools ရွေးရန် {#tools}

| Tools | Key | အသုံးပြုပုံနှင့် Options |
| --- | --- | --- |
| Rectangle Select | R | စတုဂံဆွဲပါ။ ထောင့်ဝိုင်းရန် Corner radius ပြင်ပါ။ |
| Oval Select | O | ဘဲဥပုံဆွဲပါ။ စက်ဝိုင်းအတွက် **1:1** ရွေးပါ။ |
| Free Select | F | Vertex များကို Click လုပ်ပါ၊ သို့မဟုတ် ဖိဆွဲပြီး Outline ဆွဲပါ။ စမှတ်ကို Click လုပ်ခြင်း သို့မဟုတ် Return နှိပ်ခြင်းဖြင့် ပိတ်ပါ။ Escape က Cancel လုပ်သည်။ |
| Magic Wand | W | ဆက်နေသော Color တူသည့်နေရာကို Click လုပ်ပါ။ Tolerance၊ Gap radius နှင့် Sample All Layers ပြင်နိုင်သည်။ |

Selection အသစ်လုပ်စဉ် Shift က ဧရိယာပေါင်းသည်၊ Cmd/Ctrl က နုတ်သည်၊ Shift+Cmd/Ctrl က ထပ်နေသောအပိုင်းသာ ယူသည်။ macOS တွင် Command၊ Windows/Linux တွင် Ctrl ကို သုံးပါ။ Rectangle သို့မဟုတ် Oval အသစ်ကို ဆွဲ၍ ပေါင်းစပ်နေစဉ် မူလ Selection outline နှင့် ဆွဲနေသော Shape ကို ရလဒ်အဆင်သင့်ဖြစ်သည်အထိ အတူပြထားသည်။

## ပုံသဏ္ဌာန်၊ Antialias နှင့် Feather {#edges}

Rectangle နှင့် Oval က Aspect ratio Settings ကို အတူသုံးသည်။ Free က Ratio ကန့်သတ်ချက် ဖယ်သည်။ Fixed ratio ရွေးလျှင် လက်ရှိပြင်နိုင်သောပုံသဏ္ဌာန်ကို အလယ်မှ ပြန်ညှိပေးသည်။ X/Y၊ Width/Height ဖြင့် နေရာနှင့် အရွယ်အစားပြင်ပါ။ အတွင်းမှဆွဲပြီး ရွှေ့ပါ၊ Handle ဖြင့် Resize လုပ်ပါ။ Field ကို အလွတ်ထားခြင်း သို့မဟုတ် Escape နှိပ်ခြင်းက Numeric edit ပြန်ထားသည်။

**Antialias edges** က မျဉ်းကွေးနှင့် ထောင့်များကို ချောစေသည်။ Feather ကဲ့သို့ ကျယ်ကျယ်ဖျော့ခြင်း မဟုတ်ပါ။ Wand တွင် Matching threshold ကို မပြောင်းဘဲ ရွေးပြီးသော Contour ကိုသာ ပြင်ပေးသည်။ Wand နှင့် Free Select တွင် Selection အသစ်များကိုသာ သက်ရောက်သည်။ Rectangle/Oval တွင် လက်ရှိ Editable curve ကိုလည်း ပြင်နိုင်သည်။

**Feather new selections** က ဧရိယာအသစ်၏ အနားကို တဖြည်းဖြည်း ဖျော့သွားစေသည်။ Radius သည် ပုံ၏ Pixel 0–128၊ စတင်တန်ဖိုး 8 ဖြစ်ပြီး မူလပိတ်ထားသည်။ ရှိပြီးသားပုံသဏ္ဌာန်ကို Move/Resize လုပ်လျှင် ထပ်ပြီး Feather မလုပ်ပါ။ ရှိပြီးသား Selection အတွက် **Select → Feather…** သုံးပါ။ **Grow…** က ချဲ့ပြီး **Shrink…** က ကျုံ့သည်။

Shape များ ပေါင်းစပ်ပြီးနောက် Rectangle Select သို့မဟုတ် Oval Select ဖြင့် ရွေးထားသောဧရိယာကို ဖိဆွဲလျှင် Mask တစ်ခုလုံး ရွှေ့နိုင်သည်။ အပေါက်များနှင့် Soft edges ကို မပြောင်းဘဲ Canvas အတွင်း အတူရွှေ့သည်။ အပေါက် သို့မဟုတ် မရွေးထားသောနေရာမှ ဆွဲလျှင် Selection အသစ် စသည်။ Complex mask တွင် Rectangle/Oval resize controls မရှိပါ။

## Magic Wand Settings {#wand}

Tolerance 0 က Color အတိအကျတူမှ ရွေးသည်။ တိုးလျှင် Color ကွာခြားမှု ပိုလက်ခံသည်။ လုံးဝ Transparent ဖြစ်သော Pixel များကို Hidden RGB မတူလည်း တူသည်ဟု တွက်သည်။ Gap radius 0 က ပိတ်ထားခြင်းဖြစ်သည်။ 1–8 က နယ်နိမိတ်အပေါက်ငယ်များ ပိတ်ပေးနိုင်သော်လည်း များလွန်းလျှင် ကျဉ်းသောနေရာများပါ ပိတ်နိုင်သည်။ Sample All Layers က မြင်ရသောပေါင်းစပ်ပုံမှ ရွေးပြီး မဖွင့်လျှင် Active Layer မှ ရွေးသည်။

**Select → Alpha to Selection** က Enabled Mask အပါအဝင် Active Layer ၏ Alpha ကို ယူသည်။ Text တွင်လည်း သုံးနိုင်သည်။ **Select None** က Selection ကန့်သတ်ချက် ဖယ်သည်။ Selection ပြောင်းမှုကို Undo လုပ်နိုင်ပြီး Save ပြီးသား Project ကို Unsaved အဖြစ် မပြောင်းပါ။

## ဆက်ဖတ်ရန် {#related}

- [Selection၊ Antialias နှင့် Feather](../../basics/selections/)
- [နောက်ခံဖယ်ရှားရန်](../../guides/remove-background/)
- [Path ဆွဲပြီး အသုံးပြုရန်](../../guides/paths/)
