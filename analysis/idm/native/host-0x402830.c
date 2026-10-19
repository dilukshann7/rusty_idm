/* REA ev_a0f115b1e65bb889cc2727d8657bc1ec542fee061f5f98fd87480246adbee8b3; IDMMsgHost.exe; decompiler output, not original source. */

void FUN_00402830(void)

{
  uint uVar1;
  int *piVar2;
  HKEY pHVar3;
  HMODULE pHVar4;
  HANDLE pvVar5;
  DWORD DVar6;
  LSTATUS LVar7;
  HRESULT HVar8;
  BOOL BVar9;
  HANDLE pvVar10;
  DWORD DVar11;
  int iVar12;
  LPCVOID pvVar13;
  int iVar14;
  uint uVar15;
  HKEY pHVar16;
  HKEY pHVar17;
  wchar_t *pwVar18;
  HANDLE *ppvVar19;
  WCHAR *pWVar20;
  uint uVar21;
  int iVar22;
  uint *this;
  HKEY *ppHVar23;
  _OVERLAPPED *p_Var24;
  DWORD *pSessionId;
  uint uStack_22c;
  undefined1 local_218 [8];
  undefined1 local_210 [8];
  undefined1 local_208 [8];
  undefined1 local_200 [8];
  undefined1 local_1f8 [8];
  undefined1 local_1f0 [8];
  undefined1 local_1e8 [8];
  undefined1 local_1e0 [8];
  undefined1 local_1d8 [8];
  undefined1 local_1d0 [8];
  undefined1 local_1c8 [8];
  undefined1 local_1c0 [8];
  int local_1b8 [6];
  HANDLE local_1a0;
  uint local_19c;
  _OVERLAPPED local_198;
  undefined4 local_184;
  HANDLE local_180;
  HANDLE local_17c;
  undefined4 local_178;
  char local_174 [4];
  HANDLE local_170;
  uint local_16c;
  _OVERLAPPED local_168;
  undefined4 local_154;
  undefined4 local_150;
  undefined4 local_14c;
  undefined4 local_148;
  char local_144 [4];
  int *local_140;
  HANDLE local_13c;
  undefined4 local_138;
  undefined4 local_134;
  undefined4 local_130;
  undefined4 local_12c;
  undefined4 local_128;
  HANDLE local_124;
  undefined4 local_120;
  undefined4 local_11c;
  undefined4 local_118;
  undefined4 local_114;
  undefined2 local_110;
  HANDLE local_10c;
  undefined4 local_108;
  undefined4 local_104;
  undefined4 local_100;
  undefined4 local_fc;
  undefined4 local_f8;
  HANDLE local_f4;
  undefined4 local_f0;
  HANDLE local_ec;
  HANDLE local_e8;
  undefined4 local_e4;
  char local_e0;
  int *local_dc;
  HANDLE local_d8;
  DWORD local_d4;
  uint local_d0;
  HANDLE local_c8;
  int *local_c4;
  HKEY local_c0;
  HKEY local_bc;
  uint local_b8 [4];
  HANDLE local_a8 [4];
  WCHAR local_98 [27];
  wchar_t local_62;
  undefined1 local_60 [72];
  uint local_18;
  undefined1 *local_14;
  void *local_10;
  undefined1 *puStack_c;
  undefined4 local_8;
  
  local_8 = 0xffffffff;
  puStack_c = &LAB_0040392b;
  local_10 = ExceptionList;
  uStack_22c = DAT_00406000 ^ (uint)&stack0xfffffffc;
  local_14 = (undefined1 *)&uStack_22c;
  ExceptionList = &local_10;
  local_18 = uStack_22c;
  pHVar4 = GetModuleHandleW(L"KERNEL32.DLL");
  if (pHVar4 != (HMODULE)0x0) {
    DAT_004066ac = GetProcAddress(pHVar4,"GetNamedPipeServerProcessId");
    DAT_004066b0 = GetProcAddress(pHVar4,"GetNamedPipeClientProcessId");
  }
  if (((DAT_004066ac == (FARPROC)0x0) || (DAT_004066b0 == (FARPROC)0x0)) &&
     (pHVar4 = LoadLibraryW(L"PSAPI.DLL"), pHVar4 != (HMODULE)0x0)) {
    DAT_004066b4 = GetProcAddress(pHVar4,"GetProcessImageFileNameW");
  }
  pwVar18 = L"\\\\.\\pipe\\IDMNetworkMonitor.";
  pWVar20 = local_98;
  for (iVar14 = 0xe; iVar14 != 0; iVar14 = iVar14 + -1) {
    *(undefined4 *)pWVar20 = *(undefined4 *)pwVar18;
    pwVar18 = pwVar18 + 2;
    pWVar20 = pWVar20 + 2;
  }
  memset(local_60,0,0x48);
  local_d8 = (HANDLE)0x0;
  local_1b8[0] = 0;
  local_1b8[1] = 0;
  local_1b8[2] = 0;
  FUN_00401c70(local_1b8,0x100000);
  local_8 = 0;
  local_1b8[3] = 0;
  local_1b8[4] = 0;
  local_1b8[5] = 0;
  FUN_00401c70(local_1b8 + 3,0x10000);
  local_1a0 = (HANDLE)0x0;
  local_198.Internal = 0;
  local_198.InternalHigh = 0;
  local_198.u.s.Offset = 0;
  local_198.u.s.OffsetHigh = 0;
  local_198.hEvent = (HANDLE)0x0;
  local_184 = 0;
  local_180 = (HANDLE)0x0;
  local_17c = (HANDLE)0x0;
  local_178 = 0;
  local_174[0] = '\0';
  local_174[1] = '\0';
  local_170 = (HANDLE)0x0;
  local_16c = 0;
  local_168.Internal = 0;
  local_168.InternalHigh = 0;
  local_168.u.s.Offset = 0;
  local_168.u.s.OffsetHigh = 0;
  local_168.hEvent = (HANDLE)0x0;
  local_154 = 0;
  local_150 = 0;
  local_14c = 0;
  local_148 = 0;
  local_144[0] = '\0';
  local_140 = local_1b8;
  local_134 = 0;
  local_130 = 0;
  local_12c = 0;
  local_128 = 0;
  local_124 = (HANDLE)0x0;
  local_120 = 0;
  local_11c = 0;
  local_118 = 0;
  local_114 = 0;
  local_110 = 0;
  local_10c = (HANDLE)0x0;
  local_108 = 0;
  local_104 = 0;
  local_100 = 0;
  local_fc = 0;
  local_f8 = 0;
  local_f4 = (HANDLE)0x0;
  local_f0 = 0;
  local_ec = (HANDLE)0x0;
  local_e8 = (HANDLE)0x0;
  local_e4 = 0;
  local_e0 = '\0';
  local_dc = local_1b8 + 3;
  local_19c = 0;
  local_13c = (HANDLE)0x0;
  local_138 = 0;
  local_8 = CONCAT31(local_8._1_3_,2);
  pvVar5 = OpenMutexW(0,0,L"Local\\IDMEventMonitor");
  if (pvVar5 == (HANDLE)0x0) {
    DVar6 = GetLastError();
    if (DVar6 == 2) {
      local_c0 = (HKEY)0x0;
      local_c8 = (HANDLE)0x4;
      LVar7 = RegOpenKeyExW((HKEY)0x80000002,L"Software\\Internet Download Manager",0,0x20019,
                            &local_bc);
      if (LVar7 == 0) {
        RegQueryValueExW(local_bc,L"InstallStatus",(LPDWORD)0x0,(LPDWORD)0x0,(LPBYTE)&local_c0,
                         (LPDWORD)&local_c8);
        RegCloseKey(local_bc);
        if (((local_c0 == (HKEY)0x0) || (local_c0 == (HKEY)0x3)) &&
           (HVar8 = FUN_00403750(), -1 < HVar8)) {
          FUN_00402800(local_1c0,"Launched manager via COM",0);
                    /* WARNING: Subroutine does not return */
          _CxxThrowException(local_1c0,(ThrowInfo *)&DAT_00404e44);
        }
      }
    }
  }
  else {
    CloseHandle(pvVar5);
  }
  pSessionId = &local_d4;
  local_d4 = 0;
  DVar6 = GetCurrentProcessId();
  BVar9 = ProcessIdToSessionId(DVar6,pSessionId);
  if (BVar9 == 0) {
    FUN_00402800(local_1c8,"Cannot determine session Id",0xffffffff);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_1c8,(ThrowInfo *)&DAT_00404e44);
  }
  _itow(local_d4,&local_62,10);
  pvVar5 = GetStdHandle(0xfffffff6);
  local_c8 = pvVar5;
  pvVar10 = GetStdHandle(0xfffffff5);
  if ((pvVar5 == (HANDLE)0xffffffff) || (pvVar10 == (HANDLE)0xffffffff)) {
    FUN_00402800(local_218,"Cannot open stdio",0xffffffff);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_218,(ThrowInfo *)&DAT_00404e44);
  }
  local_d8 = CreateFileW(local_98,0xc0000000,0,(LPSECURITY_ATTRIBUTES)0x0,3,0x60000000,(HANDLE)0x0);
  if (local_d8 == (HANDLE)0xffffffff) {
    FUN_00402800(local_1d0,"Cannot open pipe",0xffffffff);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_1d0,(ThrowInfo *)&DAT_00404e44);
  }
  uVar21 = 0;
  ppvVar19 = &local_168.hEvent;
  local_1a0 = pvVar5;
  local_170 = local_d8;
  local_13c = local_d8;
  local_10c = pvVar10;
  do {
    pvVar5 = CreateEventW((LPSECURITY_ATTRIBUTES)0x0,1,0,(LPCWSTR)0x0);
    pvVar10 = CreateEventW((LPSECURITY_ATTRIBUTES)0x0,1,0,(LPCWSTR)0x0);
    if ((pvVar5 == (HANDLE)0x0) || (pvVar10 == (HANDLE)0x0)) {
      FUN_00402800(local_210,"Cannot create events",0xffffffff);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(local_210,(ThrowInfo *)&DAT_00404e44);
    }
    ppvVar19[-0xc] = pvVar5;
    uVar21 = uVar21 + 1;
    *ppvVar19 = pvVar10;
    ppvVar19 = ppvVar19 + 0x19;
  } while (uVar21 < 2);
  pvVar5 = CreateSemaphoreW((LPSECURITY_ATTRIBUTES)0x0,0,1,(LPCWSTR)0x0);
  pvVar10 = CreateSemaphoreW((LPSECURITY_ATTRIBUTES)0x0,0,1,(LPCWSTR)0x0);
  if ((pvVar5 == (HANDLE)0x0) || (pvVar10 == (HANDLE)0x0)) {
    FUN_00402800(local_208,"Cannot create semaphores",0xffffffff);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_208,(ThrowInfo *)&DAT_00404e44);
  }
  local_17c = pvVar5;
  local_e8 = pvVar10;
  pvVar5 = CreateThread((LPSECURITY_ATTRIBUTES)0x0,0,FUN_004034a0,&local_198,0,(LPDWORD)0x0);
  if (pvVar5 == (HANDLE)0x0) {
    FUN_00402800(local_1d8,"Cannot create input thread",0xffffffff);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_1d8,(ThrowInfo *)&DAT_00404e44);
  }
  pvVar10 = CreateThread((LPSECURITY_ATTRIBUTES)0x0,0,FUN_004034a0,&local_104,0,(LPDWORD)0x0);
  if (pvVar10 == (HANDLE)0x0) {
    FUN_00402800(local_1e0,"Cannot create output thread",0xffffffff);
                    /* WARNING: Subroutine does not return */
    _CxxThrowException(local_1e0,(ThrowInfo *)&DAT_00404e44);
  }
  local_180 = pvVar5;
  local_ec = pvVar10;
  FUN_00403530(local_140,local_c8);
  local_19c = local_140[1];
LAB_00402cf0:
  if (local_174[0] != '\0') {
    local_b8[0] = 0;
    local_a8[0] = local_198.hEvent;
  }
  DVar6 = (DWORD)(local_174[0] != '\0');
  if (local_144[0] != '\0') {
    local_b8[DVar6] = 1;
    local_a8[DVar6] = local_168.hEvent;
    DVar6 = DVar6 + 1;
  }
  if ((char)local_110 != '\0') {
    local_b8[DVar6] = 2;
    local_a8[DVar6] = local_124;
    DVar6 = DVar6 + 1;
  }
  if (local_e0 != '\0') {
    local_b8[DVar6] = 3;
    local_a8[DVar6] = local_f4;
    DVar6 = DVar6 + 1;
  }
  iVar14 = 0;
  while (iVar14 < (int)DVar6) {
    DVar11 = WaitForMultipleObjects(DVar6,local_a8,0,(iVar14 != 0) - 1);
    if (DVar11 == 0xffffffff) {
      FUN_00402800(local_1e8,"Asynchronous I/O failed",0xffffffff);
                    /* WARNING: Subroutine does not return */
      _CxxThrowException(local_1e8,(ThrowInfo *)&DAT_00404e44);
    }
    if (DVar11 == 0x102) break;
    ResetEvent(local_a8[DVar11]);
    uVar21 = local_b8[DVar11] >> 1;
    if ((~(byte)local_b8[DVar11] & 1) == 0) {
      BVar9 = GetOverlappedResult((&local_170)[uVar21 * 0x19],&local_168 + uVar21 * 5,
                                  (LPDWORD)&local_c0,0);
      if (BVar9 == 0) goto LAB_00402e5a;
      (&local_16c)[uVar21 * 0x19] = (int)&local_c0->unused + (&local_16c)[uVar21 * 0x19];
      iVar14 = iVar14 + 1;
      local_144[uVar21 * 100] = '\0';
    }
    else {
      BVar9 = GetOverlappedResult((&local_1a0)[uVar21 * 0x19],&local_198 + uVar21 * 5,
                                  (LPDWORD)&local_c0,0);
      if (BVar9 == 0) {
LAB_00402e5a:
        DVar11 = GetLastError();
        if (DVar11 != 0x3e4) {
          FUN_00402800(local_1f0,"Overlapped I/O failed",0xffffffff);
                    /* WARNING: Subroutine does not return */
          _CxxThrowException(local_1f0,(ThrowInfo *)&DAT_00404e44);
        }
      }
      else {
        local_c4 = (&local_140)[uVar21 * 0x19];
        local_bc = local_c0;
        iVar12 = FUN_00401c70(local_c4,(int)local_c0);
        if (iVar12 != 0) {
          local_c4[1] = (int)&local_bc->unused + local_c4[1];
        }
        local_174[uVar21 * 100] = '\0';
      }
      iVar14 = iVar14 + 1;
    }
  }
  local_bc = (HKEY)0x0;
LAB_00402e6f:
  pHVar16 = local_bc;
LAB_00402e75:
  do {
    if ((HKEY)0x1 < pHVar16) goto LAB_00402cf0;
    iVar14 = (int)pHVar16 * 100;
    pHVar17 = pHVar16;
    if ((local_174[iVar14 + 1] == '\0') && (local_174[iVar14] == '\0')) {
      local_c4 = (int *)0x1;
LAB_00402eb0:
      this = (uint *)(&local_140)[(int)pHVar16 * 0x19];
      uVar21 = (&local_19c)[(int)pHVar16 * 0x19];
      uVar1 = this[1];
      if (uVar21 == 0) {
        if (uVar1 == 4) {
          local_c0 = *(HKEY *)*this;
          (&local_19c)[(int)pHVar16 * 0x19] = (uint)(*(HKEY *)*this + 1);
          if ((pHVar17 != (HKEY)0x0) && (iVar12 = FUN_00401c70(this,1), iVar12 != 0)) {
            this[1] = this[1] + 1;
            (&local_19c)[(int)pHVar16 * 0x19] = (&local_19c)[(int)pHVar16 * 0x19] + 1;
          }
          this = (uint *)(&local_140)[(int)pHVar16 * 0x19];
        }
        else {
          local_c0 = (HKEY)(4 - uVar1);
        }
LAB_00403073:
        DVar6 = 0;
        p_Var24 = &local_198 + (int)pHVar16 * 5;
        ppHVar23 = &local_c0;
        pHVar17 = local_c0;
        pvVar13 = (LPCVOID)FUN_00401c70(this,(int)local_c0);
        local_c4 = (int *)FUN_00403410((&local_1a0)[(int)pHVar16 * 0x19],pvVar13,(DWORD)pHVar17,
                                       (LPDWORD)ppHVar23,p_Var24,DVar6);
        pHVar3 = local_c0;
        if (local_c4 == (int *)0x0) goto LAB_00403142;
        piVar2 = (&local_140)[(int)pHVar16 * 0x19];
        iVar12 = FUN_00401c70(piVar2,(int)local_c0);
        pHVar17 = local_bc;
        if (iVar12 != 0) {
          piVar2 = piVar2 + 1;
          *piVar2 = (int)&pHVar3->unused + *piVar2;
        }
        goto LAB_00402eb0;
      }
      local_c0 = (HKEY)(uVar21 - uVar1);
      if (0 < (int)(uVar21 - uVar1)) goto LAB_00403073;
      if (local_c4 != (int *)0x0) {
        local_174[iVar14 + 1] = '\x01';
        local_d0 = *this;
        if (pHVar17 == (HKEY)0x0) {
          if (2 < uVar21 - 4) {
            pHVar17 = (HKEY)0x0;
            if ((*(char *)(local_d0 + 4) == '\"') && (((char *)(local_d0 + 4))[uVar21 - 5] == '\"'))
            {
              iVar12 = FUN_00402550((byte *)(local_d0 + 5),uVar21 - 6,(byte *)(local_d0 + 5),
                                    (byte *)(uVar21 - 6));
              *(int *)(local_d0 + 1) = iVar12;
              (&local_19c)[(int)pHVar16 * 0x19] = iVar12 + 5;
              (&local_16c)[(int)pHVar16 * 0x19] = 1;
              pHVar17 = local_bc;
            }
            goto LAB_00402f8f;
          }
          local_174[iVar14 + 1] = '\0';
          (&local_19c)[(int)pHVar16 * 0x19] = 0;
          this[1] = 0;
          pHVar16 = pHVar17;
          goto LAB_00402e75;
        }
        uVar21 = uVar21 - 5;
        local_d0 = FUN_00402420((byte *)(local_d0 + 5),uVar21,0);
        local_c4 = (&local_140)[(int)pHVar16 * 0x19];
        iVar22 = (local_d0 - uVar21) + 1;
        iVar12 = FUN_00401c70(local_c4,iVar22);
        if (iVar12 != 0) {
          local_c4[1] = local_c4[1] + iVar22;
        }
        iVar12 = *(&local_140)[(int)pHVar16 * 0x19];
        if (uVar21 < local_d0) {
          uVar21 = FUN_00402420((byte *)(iVar12 + 5),uVar21,(&local_140)[(int)pHVar16 * 0x19][2] - 5
                               );
        }
        *(undefined1 *)(uVar21 + 5 + iVar12) = 0x22;
        *(undefined1 *)(iVar12 + 4) = 0x22;
        *(uint *)*(&local_140)[(int)pHVar16 * 0x19] = uVar21 + 2;
        (&local_19c)[(int)pHVar16 * 0x19] = uVar21 + 6;
        pHVar17 = local_bc;
        goto LAB_00402f8f;
      }
LAB_00403142:
      DVar6 = GetLastError();
      if (DVar6 != 0x3e5) {
        FUN_00402800(local_1f8,"Read operation failed",0xffffffff);
                    /* WARNING: Subroutine does not return */
        _CxxThrowException(local_1f8,(ThrowInfo *)&DAT_00404e44);
      }
      local_174[iVar14] = '\x01';
      pHVar17 = local_bc;
    }
LAB_00402f8f:
    if (((local_144[iVar14] != '\0') || ((&local_19c)[(int)pHVar16 * 0x19] == 0)) ||
       (local_174[iVar14] != '\0')) break;
    while( true ) {
      piVar2 = (&local_140)[(int)pHVar16 * 0x19];
      uVar21 = (&local_19c)[(int)pHVar16 * 0x19];
      uVar1 = (&local_16c)[(int)pHVar16 * 0x19];
      uVar15 = uVar21;
      if ((uint)piVar2[1] <= uVar21) {
        uVar15 = piVar2[1];
      }
      local_c8 = (HANDLE)(uVar15 - uVar1);
      if ((int)local_c8 < 1) break;
      BVar9 = FUN_00403410((&local_170)[(int)pHVar16 * 0x19],(LPCVOID)(*piVar2 + uVar1),
                           (DWORD)local_c8,(LPDWORD)&local_c8,&local_168 + (int)pHVar16 * 5,1);
      if (BVar9 == 0) {
        DVar6 = GetLastError();
        if (DVar6 != 0x3e5) {
          FUN_00402800(local_200,"Write operation failed",0xffffffff);
                    /* WARNING: Subroutine does not return */
          _CxxThrowException(local_200,(ThrowInfo *)&DAT_00404e44);
        }
        local_144[iVar14] = '\x01';
        pHVar17 = local_bc;
        goto LAB_0040317f;
      }
      (&local_16c)[(int)pHVar16 * 0x19] = (&local_16c)[(int)pHVar16 * 0x19] + (int)local_c8;
    }
    pHVar17 = local_bc;
    if (uVar1 < uVar21) break;
    local_174[iVar14 + 1] = '\0';
    (&local_19c)[(int)pHVar16 * 0x19] = 0;
    (&local_16c)[(int)pHVar16 * 0x19] = 0;
    piVar2[1] = 0;
    pHVar16 = local_bc;
  } while( true );
LAB_0040317f:
  local_bc = (HKEY)((int)&pHVar17->unused + 1);
  goto LAB_00402e6f;
}


