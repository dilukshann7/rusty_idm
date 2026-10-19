/* REA ev_4260206edd9526ef507ad37671e34e2015d66acbe5be12a6d75a7d59df5f4da6; IDMan.exe; decompiler output, not original source. */

void FUN_0049d300(BYTE *param_1)

{
  BYTE BVar1;
  LSTATUS LVar2;
  BYTE *pBVar3;
  HKEY local_8;
  DWORD local_4;
  
  LVar2 = RegCreateKeyExA((HKEY)&DAT_80000001,"Software\\DownloadManager\\Queue",0,"",0,0xf003f,
                          (LPSECURITY_ATTRIBUTES)0x0,&local_8,&local_4);
  if ((LVar2 == 0) && (local_8 != (HKEY)0x0)) {
    pBVar3 = param_1;
    do {
      BVar1 = *pBVar3;
      pBVar3 = pBVar3 + 1;
    } while (BVar1 != '\0');
    RegSetValueExA(local_8,"Queue",0,1,param_1,(DWORD)(pBVar3 + (1 - (int)(param_1 + 1))));
    RegCloseKey(local_8);
  }
  return;
}


