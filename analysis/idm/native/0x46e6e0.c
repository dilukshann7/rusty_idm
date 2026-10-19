/* REA ev_6667a925440caa2f9e8fc26803a3d403d92654f2bae6da9de2dbdd2741d23951; IDMan.exe; decompiler output, not original source. */

void FUN_0046e6e0(void)

{
  short *psVar1;
  wchar_t wVar2;
  WCHAR WVar3;
  LSTATUS LVar4;
  int iVar5;
  BOOL BVar6;
  WCHAR *pWVar7;
  undefined2 *puVar8;
  wchar_t *pwVar9;
  wchar_t *_Str1;
  undefined4 *puVar10;
  uint uStack_2d4;
  DWORD local_2c0 [2];
  undefined1 *local_2b8;
  void *local_2b4;
  undefined1 *puStack_2b0;
  int local_2ac;
  wchar_t *local_2a8;
  wchar_t local_2a4 [2];
  undefined4 local_2a0;
  wchar_t *local_29c;
  wchar_t local_298 [2];
  undefined1 local_294 [4];
  wchar_t local_290;
  undefined1 auStack_28e [518];
  WCHAR local_88;
  undefined1 local_86 [126];
  uint local_8;
  
  puStack_2b0 = &LAB_00650070;
  local_2b4 = ExceptionList;
  uStack_2d4 = DAT_007608e0 ^ (uint)&local_2a8;
  local_2b8 = (undefined1 *)&uStack_2d4;
  ExceptionList = &local_2b4;
  local_2ac = 0;
  local_2c0[0] = 0x208;
  local_8 = uStack_2d4;
  LVar4 = RegQueryValueExW(DAT_00765c48,L"TempPath",(LPDWORD)0x0,(LPDWORD)0x0,(LPBYTE)&local_290,
                           local_2c0);
  if ((LVar4 == 0) && (iVar5 = __wchdir(&local_290), iVar5 == 0)) {
    pwVar9 = &local_290;
    do {
      wVar2 = *pwVar9;
      pwVar9 = pwVar9 + 1;
    } while (wVar2 != L'\0');
    if (*(short *)(local_294 + ((int)pwVar9 - (int)auStack_28e >> 1) * 2 + 2) != 0x5c) {
      puVar10 = (undefined4 *)(local_294 + 2);
      do {
        psVar1 = (short *)((int)puVar10 + 2);
        puVar10 = (undefined4 *)((int)puVar10 + 2);
      } while (*psVar1 != 0);
      *puVar10 = 0x5c;
    }
    local_2c0[1] = 0x40;
    BVar6 = GetUserNameW(&local_88,local_2c0 + 1);
    if (BVar6 == 0) {
      FUN_0040f4c0(&local_88,L"default_user");
    }
    pWVar7 = &local_88;
    do {
      WVar3 = *pWVar7;
      pWVar7 = pWVar7 + 1;
    } while (WVar3 != L'\0');
    if ((int)pWVar7 - (int)local_86 >> 1 != 0) {
      local_2a8 = (wchar_t *)0x0;
      local_2a0 = 0;
      local_2a4[0] = L'\0';
      local_2ac._0_1_ = 1;
      FUN_00402320("%sDwnlData\\%s\\",&local_290,&local_88,0);
      pwVar9 = local_2a8;
      if (local_2a8 == (wchar_t *)0x0) {
        pwVar9 = local_2a4;
      }
      FUN_0046e3c0(pwVar9);
      local_29c = (wchar_t *)0x0;
      local_294 = (undefined1  [4])0x0;
      local_298[0] = L'\0';
      local_2ac = CONCAT31(local_2ac._1_3_,2);
      puVar8 = DAT_00765c9c;
      if (DAT_00765c9c == (undefined2 *)0x0) {
        puVar8 = &DAT_00765ca0;
      }
      FUN_00402320("%sDwnlData\\%s\\",puVar8,&local_88,0);
      pwVar9 = local_2a8;
      if (local_2a8 == (wchar_t *)0x0) {
        pwVar9 = local_2a4;
      }
      _Str1 = local_29c;
      if (local_29c == (wchar_t *)0x0) {
        _Str1 = local_298;
      }
      iVar5 = __wcsicmp(_Str1,pwVar9);
      if (iVar5 != 0) {
        pwVar9 = local_29c;
        if (local_29c == (wchar_t *)0x0) {
          pwVar9 = local_298;
        }
        FUN_0046e3c0(pwVar9);
      }
      local_2ac._0_1_ = 1;
      FUN_00401210();
      local_2ac = (uint)local_2ac._1_3_ << 8;
      FUN_00401210();
      local_2ac = 0xffffffff;
    }
  }
  ExceptionList = local_2b4;
  __security_check_cookie(local_8 ^ (uint)&local_2a8);
  return;
}


