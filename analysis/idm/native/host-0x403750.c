/* REA ev_cc891bc113a3842ff5a7a9de7cc10618c7623af8a5b9c084d3d7ceab54b7c24b; IDMMsgHost.exe; decompiler output, not original source. */

HRESULT FUN_00403750(void)

{
  HRESULT HVar1;
  _union_2683 local_28;
  _union_2683 local_18;
  int *local_8;
  
  HVar1 = CoInitialize((LPVOID)0x0);
  if (-1 < HVar1) {
    local_8 = (int *)0x0;
    HVar1 = CoCreateInstance((IID *)&DAT_00404648,(LPUNKNOWN)0x0,4,(IID *)&DAT_00404638,&local_8);
    if ((HVar1 == 0) && (local_8 != (int *)0x0)) {
      VariantInit((VARIANTARG *)&local_28.n2);
      VariantInit((VARIANTARG *)&local_18.n2);
      HVar1 = (**(code **)(*local_8 + 0x10))
                        (local_8,L"idmcmd://startnetmon/",0,0,0,0,0,0,0,8,local_28._0_4_,
                         local_28.decVal.Hi32,local_28._8_4_,local_28._12_4_,local_18._0_4_,
                         local_18.decVal.Hi32,local_18._8_4_,local_18._12_4_);
      if (HVar1 == -0x7fffbfff) {
        HVar1 = 1;
      }
    }
    CoUninitialize();
  }
  return HVar1;
}


