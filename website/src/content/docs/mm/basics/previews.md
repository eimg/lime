---
title: "Preview၊ Apply၊ Cancel နှင့် Undo"
description: "Draft နှင့် Commit လုပ်ထားသော Edit ခွဲခြားပြီး Numeric field၊ Text နှင့် Preview ကို အဆုံးသတ်နည်း။"
translationKey: "basics/previews"
sourceRevision: "1.2.3-preview-1"
translatedRevision: "1.2.3-preview-1"
---

## Tool preview အဆုံးသတ်ရန် {#tool-previews}

Crop၊ Gradient၊ Scale Layer၊ Rotate၊ Flip နှင့် Perspective က Document မပြောင်းမီ Canvas ပေါ်တွင် Draft ပြပေးသည်။

1. Preview ကို လိုသလောက် ပြန်ညှိပါ။
2. Commit လုပ်ရန် **Apply**၊ Draft ပယ်ရန် **Cancel** ကို ရွေးပါ။ Canvas ကို Focus ထားစဉ် **Return** နှင့် **Escape** ဖြင့်လည်း လုပ်နိုင်သည်။ Crop အတွက် **Image → Apply Crop** လည်း ရှိသည်။
3. Apply ပြီးသော Edit ကို ပြန်ယူလိုလျှင် **Edit → Undo** ကို သုံးပါ။

Scale နှင့် Warp tools က Preview ပြန်ညှိနေစဉ် မူလ Pixel များကို ထိန်းထားသည်။ Apply မလုပ်မီ ထပ်ပြင်ခြင်းကြောင့် Layer ကို အကြိမ်ကြိမ် Resample မလုပ်ပါ။ Tool၊ Layer သို့မဟုတ် Tab ပြောင်းလျှင် ထို Transform preview ကို Cancel လုပ်သည်။ Apply/Cancel ပြီးလျှင် Transform controls ပျောက်သည်။ Session အသစ် စရန် Tool ကို ပြန်ရွေးပါ။

Apply နှိပ်ပြီးနောက် Edit အပြီးသတ်ကို ပြင်ဆင်နေစဉ် Canvas ပေါ်ရှိ Preview ကို ဆက်ပြထားသည်။

## Numeric field အတွင်း Edit {#fields}

Numeric field ထဲရှိ **Return** က ပုံမှန်အားဖြင့် ထို Field edit ကိုသာ အဆုံးသတ်သည်။ Scale/Rotate/Perspective session ကို Apply မလုပ်ပါ။ စာသားအလွတ်ထားခြင်း သို့မဟုတ် **Escape** က Field စပြင်စဉ် တန်ဖိုးကို ပြန်ထားပြီး အရင် Preview ပြင်ဆင်မှုများကို ထိန်းသည်။ Crop coordinates ပြင်လျှင် Draft က ချက်ချင်း ပြောင်းသည်။

Field edit ကို အဆုံးသတ်ပြီး Preview ၏ **Apply** ကို သုံးပါ။ Save နှင့် အချို့ Commands က Inline edit ကို အရင် အဆုံးသတ်ပေးသည်။ မပြီးသေးသော Canvas preview ကိုတော့ Apply သို့မဟုတ် Cancel လုပ်ရမည်။

Rectangle Select ၏ Radius **− / +** ကို မြန်မြန်ဆက်နှိပ်လည်း နှိပ်သည့်အစီအစဉ်အတိုင်း တွက်ပေးသည်။ Selection ပြောင်းနေစဉ် နောက်ဆုံးတောင်းထားသော Radius ကို ကိန်းဂဏန်းဖြင့် ပြထားသည်။

## Text နှင့် Stroke {#text-strokes}

Text တွင် လုပ်ပုံကွာသည်။ **Escape**၊ **Cmd/Ctrl+Enter**၊ Text အပြင်ဘက် နှိပ်ခြင်း သို့မဟုတ် Tool/Layer/Tab ပြောင်းခြင်းက Draft ကို Commit လုပ်သည်။ Escape က ရိုက်ထားသော Text ကို မပယ်ပါ။ Undo က Typing history ကို အရင်ပြန်သွားပြီးမှ Document history သို့ ဆက်သည်။ Text commit မအောင်မြင်လျှင် Draft ကို ဆက်ပြင်နိုင်ရန် ထားပေးသည်။

ပုံဆွဲခြင်းတွင် Pointer လွှတ်သောအခါ Stroke ကို Commit လုပ်သည်။ မပြီးသေးသော Stroke ကို **Escape** ဖြင့် Cancel လုပ်နိုင်သည်။ Pointer ဖိထားစဉ် Dab များ ထပ်နေသော်လည်း Stroke တစ်ခုအတွက် History တစ်ဆင့်သာ ဖြစ်သည်။

## Effect dialogs နှင့် Saved state {#dialogs-history}

Effect dialog ရှိ **Before/After** က Parameters မပြောင်းဘဲ ပုံမြင်ရပုံကို နှိုင်းယှဉ်ပေးသည်။ Before ပြထားသော်လည်း Apply က ရွေးထားသော Parameters ကို သုံးသည်။ **Cancel** က Pixel မပြောင်းဘဲ ပိတ်သည်။ **Reset** က မူလ Defaults ပြန်ထားပြီး After ကို ပြသည်။

Save လုပ်ထားသော Document state သို့ Undo ပြန်ရောက်လျှင် Unsaved marker ပျောက်သည်။ Export က Editable project ကို Save လုပ်ခြင်း မဟုတ်ပါ။ Selection သာ ပြောင်းလျှင် Project Save မလိုပါ။ Layer၊ Mask၊ Path နှင့် Guide ပြောင်းလျှင် Save လုပ်ရန် လိုသည်။

Apply နှိပ်ပြီးနောက် Effects နှင့် Text Along Path dialog များတွင် Edit အပြီးသတ် အဆင်သင့်ဖြစ်သည်အထိ **Applying…** ဟု ပြထားသည်။ ထိုအချိန်တွင် Controls ကို ခဏ သုံးမရပါ။

## ဆက်ဖတ်ရန် {#related}

- [Color adjustments နှင့် Filters](../../reference/effects/)
- [Layer transforms](../../reference/transforms/)
- [Project Save နှင့် ပုံ Export](../../reference/files/)
