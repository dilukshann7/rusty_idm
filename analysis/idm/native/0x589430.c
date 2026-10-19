/* REA ev_d7aca5af011cb3debb8cba2457f8315b6bec33bb7a174c06c1a7d9c924b5c566; IDMan.exe; decompiler output, not original source. */

/* WARNING: Removing unreachable block (ram,0x00589cb7) */
/* WARNING: Removing unreachable block (ram,0x00589cc7) */

void FUN_00589430(int param_1)

{
  char cVar1;
  undefined4 uVar2;
  uint uVar3;
  undefined1 *puVar4;
  longlong lVar5;
  longlong lVar6;
  uint *puVar7;
  char *pcVar8;
  int iVar9;
  undefined *puVar10;
  undefined4 *puVar11;
  int *piVar12;
  int iVar13;
  undefined4 uVar14;
  char *pcVar15;
  undefined2 *puVar16;
  int in_ECX;
  char *pcVar17;
  int iVar18;
  uint uVar19;
  bool bVar20;
  bool bVar21;
  longlong lVar22;
  uint uStack_21c;
  char *local_1f8;
  undefined1 *local_1f4;
  undefined2 *local_1e4;
  undefined4 local_1dc;
  undefined4 local_1d8;
  undefined4 local_1d4;
  char *local_1d0;
  undefined4 local_1cc;
  undefined4 local_1c8;
  undefined4 local_1c4;
  undefined4 local_1c0;
  undefined4 local_1bc;
  char *local_1b8;
  int local_1b4;
  undefined4 local_1b0;
  undefined1 local_1a9;
  undefined4 local_1a8;
  undefined4 local_1a4;
  char local_19e;
  char local_19d;
  int local_19c [2];
  char *local_194;
  uint local_190;
  undefined1 *local_18c;
  void *local_188;
  undefined1 *puStack_184;
  int local_180;
  CHAR local_17c [96];
  undefined1 local_11c [36];
  undefined1 local_f8 [36];
  char local_d4 [13];
  undefined2 local_c7 [57];
  char local_54 [16];
  CHAR local_44 [12];
  undefined1 local_38;
  undefined4 local_37;
  undefined4 local_33;
  undefined4 local_2f;
  undefined4 local_2b;
  undefined4 local_27;
  undefined4 local_23;
  undefined4 local_1f;
  undefined4 local_1b;
  uint local_14;
  int local_10;
  uint local_8;
  
  puStack_184 = &LAB_006676bb;
  local_188 = ExceptionList;
  uStack_21c = DAT_007608e0 ^ (uint)local_17c;
  local_18c = (undefined1 *)&uStack_21c;
  ExceptionList = &local_188;
  local_190 = 0;
  local_180 = 0;
  local_19d = '\0';
  local_19e = '\0';
  *(undefined1 *)(in_ECX + 0x32c4) = 0;
  iVar9 = *(int *)(in_ECX + 200);
  pcVar8 = *(char **)(iVar9 + 0x3a0);
  pcVar17 = pcVar8 + 1;
  do {
    cVar1 = *pcVar8;
    pcVar8 = pcVar8 + 1;
  } while (cVar1 != '\0');
  local_8 = uStack_21c;
  if ((uint)((int)pcVar8 - (int)pcVar17) < 0xd) {
    puVar7 = &uStack_21c;
    if (*(char *)(iVar9 + 0x1b6) == '\0') {
      if (*(char *)(iVar9 + 0x1b7) == '\0') {
        if ((*(int *)(in_ECX + 0x32cc) == 0) ||
           (puVar7 = &uStack_21c, *(int *)(in_ECX + 0x32d0) != 0)) {
          FUN_005863f0(iVar9 + 0x1f0,*(int *)(in_ECX + 0x32cc),*(undefined4 *)(iVar9 + 0xc),
                       *(undefined4 *)(iVar9 + 0x10),in_ECX + 0x32d0,in_ECX + 13000,0);
          puVar7 = (uint *)local_18c;
        }
      }
      else {
        pcVar8 = *(char **)(iVar9 + 0x2f4);
        pcVar17 = pcVar8 + 1;
        do {
          cVar1 = *pcVar8;
          pcVar8 = pcVar8 + 1;
        } while (cVar1 != '\0');
        puVar7 = &uStack_21c;
        if (pcVar8 != pcVar17) {
          local_19d = '\x01';
          puVar7 = &uStack_21c;
        }
      }
    }
  }
  else {
    local_19e = '\x01';
    puVar7 = &uStack_21c;
  }
  local_18c = (undefined1 *)puVar7;
  uVar14 = DAT_00766c7c;
  if (param_1 != 0) {
    uVar14 = DAT_00766c78;
  }
  FUN_00565f30(uVar14,1);
  FUN_00584790();
  pcVar17 = *(char **)(in_ECX + 0x3228);
  if (pcVar17 != (char *)0x0) {
    for (; cVar1 = *pcVar17, cVar1 != '\0'; pcVar17 = pcVar17 + 1) {
      if (((((cVar1 < 'a') || ('z' < cVar1)) && ((cVar1 < 'A' || ('Z' < cVar1)))) &&
          ((((((cVar1 < '0' || ('9' < cVar1)) && (cVar1 != ' ')) &&
             ((cVar1 != '!' && (cVar1 != '#')))) && (cVar1 != '$')) &&
           ((cVar1 != '%' && (cVar1 != '&')))))) &&
         (((cVar1 != '\'' &&
           ((((((cVar1 != '*' && (cVar1 != '+')) && (cVar1 != '-')) &&
              (((cVar1 != '.' && (cVar1 != '/')) &&
               ((cVar1 != ';' && ((cVar1 != ':' && (cVar1 != '=')))))))) && (cVar1 != '?')) &&
            (((cVar1 != '@' && (cVar1 != '^')) && (cVar1 != '_')))))) &&
          ((cVar1 != '|' && (cVar1 != '~')))))) {
        *pcVar17 = '_';
      }
    }
  }
  iVar9 = FUN_00585bc0();
  if (iVar9 != 0) {
    ExceptionList = local_188;
    __security_check_cookie(local_8 ^ (uint)local_17c);
    return;
  }
  FUN_00402500();
  local_180._0_1_ = 1;
  if (*(int *)(in_ECX + 0xcc) == 0) {
    if (param_1 == 0) {
      if ((*(char *)(*(int *)(in_ECX + 200) + 0x19d) != '\0') ||
         (puVar10 = &DAT_006b696c, *(char *)(*(int *)(in_ECX + 200) + 0x1b0) != '\0')) {
        puVar10 = &DAT_006b6970;
      }
    }
    else {
      puVar10 = &DAT_006b6964;
    }
  }
  else {
    puVar10 = &DAT_006b6970;
    if (*(char *)(*(int *)(in_ECX + 0xcc) + 0x14) == '\0') {
      puVar10 = &DAT_006b696c;
    }
  }
  FUN_00402fd0(puVar10);
  if (local_19d != '\0') {
    *(undefined1 *)(in_ECX + 0x32c4) = 1;
    FUN_00402500();
    local_180._0_1_ = 2;
    FUN_00402f70(&local_194,&DAT_006b695c,*(undefined4 *)(*(int *)(in_ECX + 200) + 0x30c));
    piVar12 = (int *)(*(int *)(in_ECX + 200) + 0x30c);
    *piVar12 = *piVar12 + 1;
    local_38 = 0;
    local_37 = 0;
    local_33 = 0;
    local_2f = 0;
    local_2b = 0;
    local_27 = 0;
    local_23 = 0;
    local_1f = 0;
    local_1b = 0;
    iVar9 = *(int *)(in_ECX + 200);
    local_1b4 = *(undefined4 *)(iVar9 + 0x300);
    FUN_0056dcd0(local_1b4,*(undefined4 *)(iVar9 + 0xc),*(undefined4 *)(iVar9 + 0x2fc),
                 *(undefined4 *)(iVar9 + 0x10),*(undefined4 *)(iVar9 + 0x2f4),
                 *(undefined4 *)(iVar9 + 0x2f8),local_11c);
    iVar9 = *(int *)(in_ECX + 200);
    FUN_0056de90(local_11c,*(undefined4 *)(iVar9 + 0x2f4),local_194,*(undefined4 *)(iVar9 + 0x2f8),
                 *(undefined4 *)(iVar9 + 0x304),local_1c8,*(undefined4 *)(in_ECX + 0x3224),&local_38
                 ,local_f8);
    FUN_00402500();
    local_180._0_1_ = 3;
    FUN_00402500();
    local_180 = CONCAT31(local_180._1_3_,4);
    iVar9 = *(int *)(in_ECX + 200);
    pcVar8 = *(char **)(iVar9 + 0x304);
    pcVar17 = pcVar8 + 1;
    do {
      cVar1 = *pcVar8;
      pcVar8 = pcVar8 + 1;
    } while (cVar1 != '\0');
    if (pcVar8 != pcVar17) {
      FUN_00402f70(local_19c,"qop=%s, nc=%s, cnonce=\"%s\", ",*(undefined4 *)(iVar9 + 0x304),
                   local_194,*(undefined4 *)(iVar9 + 0x2f8));
    }
    pcVar8 = *(char **)(*(int *)(in_ECX + 200) + 0x308);
    pcVar17 = pcVar8 + 1;
    do {
      cVar1 = *pcVar8;
      pcVar8 = pcVar8 + 1;
    } while (cVar1 != '\0');
    if (pcVar8 != pcVar17) {
      FUN_00402f70(&local_1b8,", opaque=\"%s\"",*(undefined4 *)(*(int *)(in_ECX + 200) + 0x308));
    }
    pcVar17 = local_1b8;
    do {
      cVar1 = *pcVar17;
      pcVar17 = pcVar17 + 1;
    } while (cVar1 != '\0');
    if (pcVar17 == local_1b8 + 1) {
      puVar11 = (undefined4 *)FUN_004030f0(&DAT_0067e9c4);
      local_180 = CONCAT31(local_180._1_3_,5);
      local_190 = local_190 | 1;
      local_1b8 = (char *)*puVar11;
    }
    pcVar8 = *(char **)(*(int *)(in_ECX + 200) + 0x304);
    pcVar17 = pcVar8 + 1;
    do {
      cVar1 = *pcVar8;
      pcVar8 = pcVar8 + 1;
    } while (cVar1 != '\0');
    iVar9 = local_19c[0];
    if (pcVar8 == pcVar17) {
      piVar12 = (int *)FUN_004030f0(&DAT_0067e9c4);
      local_180 = 6;
      local_190 = local_190 | 2;
      iVar9 = *piVar12;
    }
    iVar13 = *(int *)(in_ECX + 200);
    FUN_00402f70(in_ECX + 13000,
                 "Authorization: Digest username=\"%s\", realm=\"%s\", nonce=\"%s\", uri=\"%s\", %sresponse=\"%s\"%s\r\n"
                 ,*(undefined4 *)(iVar13 + 0xc),*(undefined4 *)(iVar13 + 0x2fc),
                 *(undefined4 *)(iVar13 + 0x2f4),*(undefined4 *)(in_ECX + 0x3224),iVar9,local_f8,
                 local_1b8);
    local_180 = 5;
    if ((local_190 & 2) != 0) {
      local_190 = local_190 & 0xfffffffd;
      FUN_00402570();
    }
    local_180 = 4;
    if ((local_190 & 1) != 0) {
      local_190 = local_190 & 0xfffffffe;
      FUN_00402570();
    }
    local_180._0_1_ = 3;
    FUN_00402570();
    local_180._0_1_ = 2;
    FUN_00402570();
    local_180._0_1_ = 1;
    FUN_00402570();
  }
  iVar9 = *(int *)(in_ECX + 200);
  bVar21 = *(char *)(iVar9 + 0x349) != '\0';
  *(undefined4 *)(in_ECX + 0x32a0) = 0;
  *(undefined4 *)(in_ECX + 0x32a4) = 0;
  *(undefined4 *)(in_ECX + 0x68) = 0xffffffff;
  *(undefined4 *)(in_ECX + 0x6c) = 0xffffffff;
  if ((*(int *)(iVar9 + 0x340) == 8) && (iVar13 = *(int *)(in_ECX + 0xa8), iVar13 != 0)) {
    uVar19 = *(uint *)(iVar13 + 0x50);
    if (uVar19 == 0 && *(int *)(iVar13 + 0x54) == 0) goto LAB_00589943;
    iVar9 = uVar19 + *(uint *)(iVar13 + 0x48);
    iVar18 = (*(int *)(iVar13 + 0x54) + *(int *)(iVar13 + 0x4c) +
             (uint)CARRY4(uVar19,*(uint *)(iVar13 + 0x48))) - (uint)(iVar9 == 0);
    *(int *)(in_ECX + 0x68) = iVar9 + -1;
    *(int *)(in_ECX + 0x6c) = iVar18;
    _sprintf(local_d4,"Range: bytes=%I64d-%I64d\r\n",*(undefined4 *)(iVar13 + 0x48),
             *(undefined4 *)(iVar13 + 0x4c),*(undefined4 *)(in_ECX + 0x68),iVar18);
  }
  else {
LAB_00589943:
    if ((((*(int *)(iVar9 + 0x340) == 0x40) && (*(int *)(in_ECX + 0x3208) != 0)) &&
        (iVar13 = *(int *)(*(int *)(in_ECX + 0x3208) + 0x40), iVar13 != 0)) &&
       ((*(int *)(iVar13 + 0x58) != 0 && (iVar13 = *(int *)(in_ECX + 0xa8), iVar13 != 0)))) {
      *(undefined4 *)(in_ECX + 0x60) = *(undefined4 *)(iVar13 + 0x38);
      *(undefined4 *)(in_ECX + 100) = *(undefined4 *)(iVar13 + 0x3c);
      iVar9 = *(int *)(iVar13 + 0x40);
      iVar13 = *(int *)(iVar13 + 0x44);
      if ((iVar13 < 0) || ((iVar13 < 1 && (iVar9 == 0)))) {
        _sprintf(local_d4,"Range: bytes=%I64d-\r\n",*(undefined4 *)(in_ECX + 0x60),
                 *(undefined4 *)(in_ECX + 100));
      }
      else {
        *(int *)(in_ECX + 0x78) = iVar9;
        *(int *)(in_ECX + 0x7c) = iVar13;
        *(int *)(in_ECX + 0x68) = iVar9;
        *(undefined4 *)(in_ECX + 0x6c) = *(undefined4 *)(in_ECX + 0x7c);
        _sprintf(local_d4,"Range: bytes=%I64d-%I64d\r\n",*(undefined4 *)(in_ECX + 0x60),
                 *(undefined4 *)(in_ECX + 100),iVar9,*(undefined4 *)(in_ECX + 0x7c));
      }
    }
    else if (*(char *)(in_ECX + 0x320f) == '\0') {
      uVar19 = *(uint *)(in_ECX + 0x60);
      iVar13 = *(int *)(in_ECX + 100);
      if ((iVar13 < 1) && ((iVar13 < 0 || (uVar19 == 0)))) {
        if ((((*(int *)(iVar9 + 0x334) == 0x40) || (*(int *)(iVar9 + 0x334) == 0x80)) &&
            (iVar13 = *(int *)(in_ECX + 0x3208), iVar13 != 0)) &&
           (*(char *)(iVar13 + 0x25a) != '\0')) {
          uVar14 = *(undefined4 *)(iVar13 + 0x250);
          *(undefined4 *)(in_ECX + 0x78) = uVar14;
          uVar2 = *(undefined4 *)(iVar13 + 0x254);
          *(undefined4 *)(in_ECX + 0x7c) = uVar2;
          *(undefined4 *)(in_ECX + 0x68) = uVar14;
          *(undefined4 *)(in_ECX + 0x6c) = uVar2;
          _sprintf(local_d4,"Range: bytes=%I64d-%I64d\r\n",*(undefined4 *)(iVar13 + 0x248),
                   *(undefined4 *)(iVar13 + 0x24c),*(undefined4 *)(iVar13 + 0x250),
                   *(undefined4 *)(iVar13 + 0x254));
        }
        else if ((*(char *)(in_ECX + 0xe2) == '\0') && (*(char *)(iVar9 + 0x1b9) == '\0')) {
          if (((*(char *)(iVar9 + 0x198) == '\0') || (*(int *)(iVar9 + 0x314) < 0)) ||
             ((*(int *)(iVar9 + 0x314) < 1 && (*(int *)(iVar9 + 0x310) == 0)))) {
            if ((*(int *)(in_ECX + 0x3208) != 0) &&
               (*(int *)(*(int *)(in_ECX + 0x3208) + 0x44) != 0)) {
              iVar9 = *(int *)(in_ECX + 0x78);
              iVar13 = *(int *)(in_ECX + 0x7c);
              if ((-1 < iVar13) && ((0 < iVar13 || (iVar9 != 0)))) {
                *(int *)(in_ECX + 0x68) = iVar9;
                *(int *)(in_ECX + 0x6c) = iVar13;
                _sprintf(local_d4,"Range: bytes=0-%I64d\r\n",iVar9,iVar13);
                goto LAB_00589d84;
              }
            }
            _sprintf(local_d4,"");
            *(undefined1 *)(in_ECX + 0x320c) = 0;
          }
          else {
            uVar14 = *(undefined4 *)(iVar9 + 0x310);
            *(undefined4 *)(in_ECX + 0x78) = uVar14;
            uVar2 = *(undefined4 *)(iVar9 + 0x314);
            *(undefined4 *)(in_ECX + 0x7c) = uVar2;
            *(undefined4 *)(in_ECX + 0x68) = uVar14;
            *(undefined4 *)(in_ECX + 0x6c) = uVar2;
            _sprintf(local_d4,"Range: bytes=0-%I64d\r\n",*(undefined4 *)(iVar9 + 0x310),
                     *(undefined4 *)(iVar9 + 0x314));
          }
        }
        else {
          _sprintf(local_d4,"Range: bytes=0-\r\n");
        }
      }
      else {
        if (*(char *)(iVar9 + 0x36c) == '\0') {
          iVar18 = *(int *)(in_ECX + 0x3208);
          if (iVar18 == 0) {
            local_19c[0] = 0;
            iVar18 = 0;
          }
          else {
            local_19c[0] = *(int *)(iVar18 + 0xb0);
            iVar18 = *(int *)(iVar18 + 0x44);
          }
          if ((((*(int *)(in_ECX + 0x58) == 0 && *(int *)(in_ECX + 0x5c) == 0) ||
               (local_19c[0] == 0)) || (*(int *)(iVar9 + 0x318) == 0)) && (iVar18 == 0)) {
            bVar21 = *(int *)(iVar9 + 0x318) == 0 || bVar21;
            if (*(char *)(in_ECX + 0xd8) != '\0') {
              bVar20 = 0xffffffcf < uVar19;
              uVar19 = uVar19 + 0x30;
              iVar13 = iVar13 + (uint)bVar20;
            }
            *(uint *)(in_ECX + 0x32a0) = uVar19;
            *(int *)(in_ECX + 0x32a4) = iVar13;
            if (*(char *)(in_ECX + 0xdf) != '\0') {
              uVar3 = *(uint *)(in_ECX + 0x194);
              iVar13 = (int)uVar3 >> 0x1f;
              iVar9 = *(int *)(in_ECX + 0x32a4);
              if ((iVar9 < iVar13) || ((iVar9 <= iVar13 && (uVar19 < uVar3)))) {
                *(undefined1 *)(in_ECX + 0xdf) = 0;
              }
              else {
                *(uint *)(in_ECX + 0x32a0) = uVar19 - uVar3;
                *(uint *)(in_ECX + 0x32a4) = (iVar9 - iVar13) - (uint)(uVar19 < uVar3);
              }
            }
            _sprintf(local_d4,"Range: bytes=%I64d-\r\n",*(undefined4 *)(in_ECX + 0x32a0),
                     *(undefined4 *)(in_ECX + 0x32a4));
            *(undefined1 *)(in_ECX + 0x3214) = 0;
            goto LAB_00589d84;
          }
          lVar22 = FUN_0057ecf0();
          lVar5 = lVar22;
          if (*(int *)(in_ECX + 0x3208) != 0) {
            if (*(char *)(*(int *)(in_ECX + 0x3208) + 0x25f) == '\0') {
              iVar13 = *(int *)(in_ECX + 0x58) + -1;
              iVar9 = *(int *)(in_ECX + 0x5c) - (uint)(*(int *)(in_ECX + 0x58) == 0);
              lVar6 = CONCAT44(iVar9,iVar13);
              if ((lVar22 < lVar6) && (lVar5 = CONCAT44(iVar9,iVar13), lVar22 + 0x40 < lVar6)) {
                lVar5 = lVar22 + 0x40;
              }
            }
            else {
              lVar5 = lVar22 + 1;
            }
          }
          uVar19 = *(uint *)(in_ECX + 0x60);
          iVar9 = *(int *)(in_ECX + 100);
          if (*(char *)(in_ECX + 0xd8) != '\0') {
            bVar20 = 0xffffffcf < uVar19;
            uVar19 = uVar19 + 0x30;
            iVar9 = iVar9 + (uint)bVar20;
          }
          *(uint *)(in_ECX + 0x32a0) = uVar19;
          *(int *)(in_ECX + 0x32a4) = iVar9;
          if (*(char *)(in_ECX + 0xdf) != '\0') {
            local_14 = *(uint *)(in_ECX + 0x194);
            local_10 = (int)local_14 >> 0x1f;
            uVar19 = *(uint *)(in_ECX + 0x32a0);
            iVar9 = *(int *)(in_ECX + 0x32a4);
            if ((iVar9 < local_10) || ((iVar9 <= local_10 && (uVar19 < local_14)))) {
              *(undefined1 *)(in_ECX + 0xdf) = 0;
            }
            else {
              *(uint *)(in_ECX + 0x32a0) = uVar19 - local_14;
              *(uint *)(in_ECX + 0x32a4) = (iVar9 - local_10) - (uint)(uVar19 < local_14);
            }
          }
          *(longlong *)(in_ECX + 0x68) = lVar5;
          iVar13 = *(int *)(in_ECX + 0x32a4);
          uVar19 = *(uint *)(in_ECX + 0x32a0);
        }
        else {
          lVar5 = *(longlong *)(in_ECX + 0x78);
          *(undefined4 *)(in_ECX + 0x68) = *(undefined4 *)(in_ECX + 0x78);
          *(undefined4 *)(in_ECX + 0x6c) = *(undefined4 *)(in_ECX + 0x7c);
        }
        _sprintf(local_d4,"Range: bytes=%I64d-%I64d\r\n",uVar19,iVar13,(int)lVar5,
                 (int)((ulonglong)lVar5 >> 0x20));
        *(undefined1 *)(in_ECX + 0x3214) = 1;
      }
    }
    else {
      _sprintf(local_d4,"Range: bytes=0-\r\n");
    }
  }
LAB_00589d84:
  iVar9 = *(int *)(in_ECX + 0xcc);
  if (iVar9 == 0) {
    iVar13 = *(int *)(*(int *)(in_ECX + 200) + 0x14);
    local_1b4 = *(int *)(*(int *)(in_ECX + 200) + 0x24);
  }
  else {
    iVar13 = *(int *)(iVar9 + 0xc);
    local_1b4 = *(int *)(iVar9 + 0x10);
  }
  if (iVar13 == 0x50) {
    if (local_1b4 == 1) goto LAB_00589e56;
LAB_00589dc3:
    _sprintf(local_54,":%ld",iVar13);
  }
  else {
    if ((iVar13 != 0x1bb) || (local_1b4 != 2)) goto LAB_00589dc3;
LAB_00589e56:
    _sprintf(local_54,"");
  }
  FUN_00402500();
  local_180 = CONCAT31(local_180._1_3_,7);
  iVar9 = *(int *)(in_ECX + 200);
  if ((*(char *)(iVar9 + 0x35e) == '\0') || (*(char *)(iVar9 + 0x360) == '\0')) {
    if (*(char **)(iVar9 + 0x18) == (char *)0x0) {
      if (*(char *)(iVar9 + 0x36b) != '\0') {
        pcVar8 = *(char **)(iVar9 + 0x124);
        pcVar17 = pcVar8 + 1;
        do {
          cVar1 = *pcVar8;
          pcVar8 = pcVar8 + 1;
        } while (cVar1 != '\0');
        if (pcVar8 != pcVar17) {
          FUN_00402f70(&local_1a8,"Referer: %s\r\n",*(undefined4 *)(iVar9 + 0x124));
          goto LAB_00589eb0;
        }
      }
      goto LAB_00589eaa;
    }
    iVar9 = __strnicmp(*(char **)(iVar9 + 0x18),"file://",7);
    if (iVar9 == 0) goto LAB_00589eaa;
    FUN_00402f70(&local_1a8,"Referer: %s\r\n",*(undefined4 *)(*(int *)(in_ECX + 200) + 0x18));
  }
  else {
LAB_00589eaa:
    FUN_00402fd0(&DAT_0067e9c4);
  }
LAB_00589eb0:
  FUN_004030f0(&DAT_0067e9c4);
  local_180._0_1_ = 8;
  if ((*(char *)(*(int *)(in_ECX + 200) + 0x363) != '\0') &&
     (pcVar17 = *(char **)(*(int *)(in_ECX + 200) + 0x20), pcVar17 != (char *)0x0)) {
    pcVar8 = pcVar17 + 1;
    do {
      cVar1 = *pcVar17;
      pcVar17 = pcVar17 + 1;
    } while (cVar1 != '\0');
    for (iVar9 = (int)pcVar17 - (int)pcVar8; 1 < iVar9; iVar9 = iVar9 + -1) {
      pcVar17 = (char *)(*(int *)(*(int *)(in_ECX + 200) + 0x20) + -1 + iVar9);
      cVar1 = *pcVar17;
      if ((cVar1 != '\n') && (cVar1 != '\r')) {
        FUN_00402f70(&local_1d8,"If-Modified-Since: %s\r\n",
                     *(undefined4 *)(*(int *)(in_ECX + 200) + 0x20));
        break;
      }
      *pcVar17 = '\0';
    }
  }
  FUN_00402500();
  local_180 = CONCAT31(local_180._1_3_,9);
  iVar9 = *(int *)(in_ECX + 0xcc);
  if (iVar9 == 0) {
    pcVar8 = *(char **)(*(int *)(in_ECX + 200) + 0xd0);
    pcVar17 = pcVar8 + 1;
    do {
      cVar1 = *pcVar8;
      pcVar8 = pcVar8 + 1;
    } while (cVar1 != '\0');
    if (pcVar8 != pcVar17) {
      uVar14 = *(undefined4 *)(*(int *)(in_ECX + 200) + 0xd0);
      goto LAB_00589f71;
    }
LAB_00589f85:
    FUN_00402f70(&local_1a4,"application/x-www-form-urlencoded");
  }
  else {
    pcVar8 = *(char **)(iVar9 + 0x1c);
    pcVar17 = pcVar8 + 1;
    do {
      cVar1 = *pcVar8;
      pcVar8 = pcVar8 + 1;
    } while (cVar1 != '\0');
    if (pcVar8 == pcVar17) goto LAB_00589f85;
    uVar14 = *(undefined4 *)(iVar9 + 0x1c);
LAB_00589f71:
    FUN_00402f70(&local_1a4,&DAT_0067def0,uVar14);
  }
  FUN_00402500();
  local_180._0_1_ = 10;
  iVar9 = *(int *)(in_ECX + 0xcc);
  if (iVar9 == 0) {
    if (*(char *)(*(int *)(in_ECX + 200) + 0x19d) != '\0') {
      FUN_00402f70(&local_1c0,"Content-Type: %s\r\nContent-Length: %d\r\n",local_1a4,
                   *(undefined4 *)(*(int *)(*(int *)(in_ECX + 200) + 0x1a0) + -0xc));
    }
    if (*(char *)(*(int *)(in_ECX + 200) + 0x1b0) != '\0') {
      uVar14 = *(undefined4 *)(*(int *)(in_ECX + 200) + 0x1ac);
      goto LAB_00589ffd;
    }
  }
  else if (*(char *)(iVar9 + 0x14) != '\0') {
    uVar14 = *(undefined4 *)(*(int *)(iVar9 + 0x18) + -0xc);
LAB_00589ffd:
    FUN_00402f70(&local_1c0,"Content-Type: %s\r\nContent-Length: %d\r\n",local_1a4,uVar14);
  }
  FUN_00402500();
  local_180 = CONCAT31(local_180._1_3_,0xb);
  pcVar8 = *(char **)(*(int *)(in_ECX + 200) + 900);
  pcVar17 = pcVar8 + 1;
  do {
    cVar1 = *pcVar8;
    pcVar8 = pcVar8 + 1;
  } while (cVar1 != '\0');
  if (pcVar8 != pcVar17) {
    FUN_00402f70(&local_1d4,"Cookie2: %s\r\n",*(undefined4 *)(*(int *)(in_ECX + 200) + 900));
  }
  pcVar8 = *(char **)(*(int *)(in_ECX + 200) + 0x388);
  pcVar17 = pcVar8 + 1;
  do {
    cVar1 = *pcVar8;
    pcVar8 = pcVar8 + 1;
  } while (cVar1 != '\0');
  if (pcVar8 == pcVar17) {
    if ((DAT_00765c0c == (int)pcVar8 - (int)pcVar17) ||
       (iVar9 = GetLocaleInfoA(0x400,0x5c,local_17c,0x5f), iVar9 == 0)) {
      iVar9 = GetLocaleInfoA(0x400,0x59,local_44,9);
      if ((iVar9 == 0) || (iVar9 = GetLocaleInfoA(0x400,0x5a,(LPSTR)&local_14,9), iVar9 == 0)) {
        FUN_00402f70(*(int *)(in_ECX + 200) + 0x388,"en-US");
      }
      else {
        FUN_00402f70(*(int *)(in_ECX + 200) + 0x388,"%s-%s",local_44,&local_14);
      }
    }
    else {
      FUN_00402f70(*(int *)(in_ECX + 200) + 0x388,&DAT_0067def0,local_17c);
    }
    pcVar17 = *(char **)(*(int *)(in_ECX + 200) + 0x388);
    pcVar8 = pcVar17;
    do {
      cVar1 = *pcVar8;
      pcVar8 = pcVar8 + 1;
    } while (cVar1 != '\0');
    if (((2 < (uint)((int)pcVar8 - (int)(pcVar17 + 1))) &&
        (pcVar17 = _strchr(pcVar17,0x2d), pcVar17 != (char *)0x0)) &&
       (iVar9 = _isdigit((int)pcVar17[1]), iVar9 != 0)) {
      *pcVar17 = '\0';
    }
  }
  FUN_00402500();
  local_180._0_1_ = 0xc;
  pcVar8 = *(char **)(*(int *)(in_ECX + 200) + 0x388);
  pcVar17 = pcVar8 + 1;
  do {
    cVar1 = *pcVar8;
    pcVar8 = pcVar8 + 1;
  } while (cVar1 != '\0');
  if (pcVar8 != pcVar17) {
    FUN_00402f70(&local_1dc,"Accept-Language: %s\r\n",
                 *(undefined4 *)(*(int *)(in_ECX + 200) + 0x388));
  }
  FUN_00402500();
  local_180 = CONCAT31(local_180._1_3_,0xd);
  pcVar17 = _strrchr((char *)**(undefined4 **)(in_ECX + 200),0x2e);
  if ((pcVar17 != (char *)0x0) && (iVar9 = __stricmp(pcVar17,".monster"), iVar9 == 0)) {
    *(undefined1 *)(*(int *)(in_ECX + 200) + 0x369) = 1;
  }
  iVar9 = *(int *)(in_ECX + 200);
  pcVar8 = *(char **)(iVar9 + 0x390);
  pcVar17 = pcVar8 + 1;
  do {
    cVar1 = *pcVar8;
    pcVar8 = pcVar8 + 1;
  } while (cVar1 != '\0');
  if (pcVar8 == pcVar17) {
    if (*(char *)(iVar9 + 0x369) == '\0') {
      pcVar8 = *(char **)(iVar9 + 0x124);
      pcVar17 = pcVar8 + 1;
      do {
        cVar1 = *pcVar8;
        pcVar8 = pcVar8 + 1;
      } while (cVar1 != '\0');
      if (pcVar8 == pcVar17) {
        if ((*(char **)(iVar9 + 0x18) != (char *)0x0) && (*(char *)(iVar9 + 0x35e) == '\0')) {
          local_1a9 = 0;
          pcVar17 = _strrchr(*(char **)(iVar9 + 0x18),0x2e);
          if ((pcVar17 == (char *)0x0) || (iVar9 = __strnicmp(pcVar17,".swf",4), iVar9 != 0)) {
            FUN_00402500();
            local_180 = CONCAT31(local_180._1_3_,0xf);
            uVar14 = *(undefined4 *)(*(int *)(in_ECX + 200) + 0x18);
            goto LAB_0058a2b2;
          }
          local_1a9 = 1;
        }
      }
      else {
        FUN_00402500();
        local_180 = CONCAT31(local_180._1_3_,0xe);
        uVar14 = *(undefined4 *)(*(int *)(in_ECX + 200) + 0x124);
LAB_0058a2b2:
        FUN_00402f70(&local_194,&DAT_0067def0,uVar14);
        pcVar17 = _strstr(local_194,"://");
        if ((pcVar17 != (char *)0x0) &&
           (pcVar17 = _strpbrk(pcVar17 + 3,":/"), pcVar17 != (char *)0x0)) {
          *pcVar17 = '\0';
          FUN_00402f70(&local_1c4,"Origin: %s\r\n",local_194);
        }
        local_180 = CONCAT31(local_180._1_3_,0xd);
        FUN_00402570();
      }
    }
  }
  else {
    FUN_00402f70(&local_1c4,"Origin: %s\r\n",*(undefined4 *)(iVar9 + 0x390));
  }
  FUN_00402500();
  local_180 = CONCAT31(local_180._1_3_,0x10);
  iVar9 = *(int *)(in_ECX + 200);
  pcVar8 = *(char **)(iVar9 + 0x398);
  pcVar17 = pcVar8 + 1;
  do {
    cVar1 = *pcVar8;
    pcVar8 = pcVar8 + 1;
  } while (cVar1 != '\0');
  if (pcVar8 == pcVar17) {
    if (*(char *)(iVar9 + 0x36a) == '\0') {
      pcVar17 = "Accept-Charset: *\r\n";
    }
    else {
      pcVar17 = "";
    }
    FUN_00402f70(&local_1bc,pcVar17);
  }
  else {
    FUN_00402f70(&local_1bc,"Accept-Charset: %s\r\n",*(undefined4 *)(iVar9 + 0x398));
  }
  FUN_00402500();
  local_180 = CONCAT31(local_180._1_3_,0x11);
  iVar9 = *(int *)(in_ECX + 200);
  if (*(char *)(iVar9 + 0x366) == '\0') {
    if (((*(char *)(iVar9 + 0x359) == '\0') && (*(char *)(iVar9 + 0x35a) == '\0')) ||
       (*(char *)(iVar9 + 0x35c) != '\0')) {
      pcVar8 = *(char **)(iVar9 + 0x418);
      pcVar17 = pcVar8 + 1;
      do {
        cVar1 = *pcVar8;
        pcVar8 = pcVar8 + 1;
      } while (cVar1 != '\0');
      if ((pcVar8 != pcVar17) && (*(char *)(iVar9 + 0x36d) != '\0')) {
        FUN_00402f70(&local_1b0,"Accept-Encoding: %s\r\n",*(undefined4 *)(iVar9 + 0x418));
        goto LAB_0058a401;
      }
      pcVar17 = "Accept-Encoding: identity\r\n";
    }
    else {
      pcVar17 = "";
    }
  }
  else {
    pcVar17 = "Accept-Encoding: gzip, deflate\r\n";
  }
  FUN_00402f70(&local_1b0,pcVar17);
LAB_0058a401:
  FUN_00402500();
  local_180._0_1_ = 0x12;
  if (*(int *)(in_ECX + 0xcc) == 0) {
    local_19c[0] = *(int *)(*(int *)(in_ECX + 200) + 0x1a0);
  }
  else {
    local_19c[0] = *(int *)(*(int *)(in_ECX + 0xcc) + 0x18);
  }
  iVar9 = *(int *)(in_ECX + 200);
  pcVar8 = *(char **)(iVar9 + 0x39c);
  pcVar17 = pcVar8 + 1;
  do {
    cVar1 = *pcVar8;
    pcVar8 = pcVar8 + 1;
  } while (cVar1 != '\0');
  if ((pcVar8 == pcVar17) || (*(char *)(iVar9 + 0x365) == '\0')) {
    puVar11 = (undefined4 *)FUN_004030f0(&DAT_0067e9c4);
    local_180._0_1_ = 0x13;
    local_190 = local_190 | 4;
    local_1b8 = (char *)*puVar11;
  }
  else {
    local_1b8 = *(char **)(iVar9 + 0x39c);
  }
  local_194 = "Cache-Control: no-cache\r\nPragma: no-cache\r\n";
  if (!bVar21) {
    local_194 = "";
  }
  puVar4 = *(undefined1 **)(in_ECX + 0x3228);
  local_1e4 = &DAT_0068012c;
  local_1f4 = puVar4;
  if (puVar4 == (undefined1 *)0x0) {
    local_1e4 = (undefined2 *)&DAT_0067e9c4;
    local_1f4 = &DAT_0067e9c4;
  }
  local_1f8 = "Cookie: ";
  if (puVar4 == (undefined1 *)0x0) {
    local_1f8 = "";
  }
  iVar9 = *(int *)(in_ECX + 200);
  if (local_19e == '\0') {
    if (*(char *)(iVar9 + 0x1b6) == '\0') {
      local_1cc = *(undefined4 *)(in_ECX + 13000);
    }
    else {
      local_1cc = *(undefined4 *)(in_ECX + 0x3238);
    }
  }
  else {
    local_1cc = *(undefined4 *)(iVar9 + 0x3a0);
  }
  if (((*(int *)(*(int *)(in_ECX + 13000) + -0xc) == 0) || (*(char *)(iVar9 + 0x1b6) != '\0')) ||
     (pcVar17 = "Connection: Keep-Alive\r\n", *(int *)(in_ECX + 0x32cc) != 0)) {
    pcVar17 = "";
  }
  pcVar15 = *(char **)(iVar9 + 0x394);
  pcVar8 = pcVar15 + 1;
  do {
    cVar1 = *pcVar15;
    pcVar15 = pcVar15 + 1;
  } while (cVar1 != '\0');
  if (pcVar15 == pcVar8) {
    puVar11 = (undefined4 *)FUN_004030f0(&DAT_006b66c8);
    local_180._0_1_ = 0x14;
    local_180._1_3_ = 0;
    local_190 = local_190 | 8;
    uVar14 = *puVar11;
  }
  else {
    uVar14 = *(undefined4 *)(iVar9 + 0x394);
  }
  puVar11 = *(undefined4 **)(in_ECX + 0xcc);
  if (puVar11 == (undefined4 *)0x0) {
    puVar11 = (undefined4 *)FUN_004030f0(**(undefined4 **)(in_ECX + 200));
    local_180._0_1_ = 0x15;
    local_180._1_3_ = 0;
    local_190 = local_190 | 0x10;
  }
  FUN_00402f70(&local_1d0,
               "%s %s HTTP/1.1\r\nHost: %s%s\r\nUser-Agent: %s\r\nAccept: %s\r\n%s%s%s%s%s%s%s%s%s%s%s%s%s%s%s%s\r\n%s"
               ,local_1c8,*(undefined4 *)(in_ECX + 0x3224),*puVar11,local_54,
               *(undefined4 *)(*(int *)(in_ECX + 200) + 0x1a4),uVar14,local_1b0,local_1dc,local_1bc,
               local_1c4,pcVar17,local_1d8,local_d4,local_1a8,local_1cc,local_1f8,local_1f4,
               local_1e4,local_1d4,local_1c0,local_194,local_1b8,local_19c[0]);
  local_180 = 0x14;
  uVar19 = local_190;
  if ((local_190 & 0x10) != 0) {
    uVar19 = local_190 & 0xffffffef;
    local_190 = uVar19;
    FUN_00402570();
  }
  local_180 = 0x13;
  if ((uVar19 & 8) != 0) {
    uVar19 = uVar19 & 0xfffffff7;
    local_190 = uVar19;
    FUN_00402570();
  }
  local_180 = 0x12;
  if ((uVar19 & 4) != 0) {
    local_190 = uVar19 & 0xfffffffb;
    FUN_00402570();
  }
  if (((0 < *(int *)(in_ECX + 100)) ||
      ((-1 < *(int *)(in_ECX + 100) && (*(int *)(in_ECX + 0x60) != 0)))) ||
     (*(int *)(in_ECX + 0x78) != 0 || *(int *)(in_ECX + 0x7c) != 0)) {
    FUN_00402500();
    local_180._0_1_ = 0x16;
    pcVar17 = local_d4;
    do {
      cVar1 = *pcVar17;
      pcVar17 = pcVar17 + 1;
    } while (cVar1 != '\0');
    puVar16 = local_c7;
    if (pcVar17 == local_d4 + 1) {
      puVar16 = &DAT_0068012c;
    }
    FUN_00402f70(local_19c,"%I64d-%I64d, %s",*(undefined4 *)(in_ECX + 0x60),
                 *(undefined4 *)(in_ECX + 100),*(undefined4 *)(in_ECX + 0x78),
                 *(undefined4 *)(in_ECX + 0x7c),puVar16);
    FUN_005656c0(local_19c[0]);
    local_180 = CONCAT31(local_180._1_3_,0x12);
    FUN_00402570();
  }
  *(undefined4 *)(in_ECX + 0x325c) = 0;
  *(undefined1 *)(in_ECX + 0xe0) = 0;
  *(undefined4 *)(in_ECX + 0x3204) = 1;
  FUN_00565920();
  *(undefined4 *)(in_ECX + 0xa4) = 4;
  if (local_1b4 == 2) {
    pcVar17 = local_1d0;
    do {
      cVar1 = *pcVar17;
      pcVar17 = pcVar17 + 1;
    } while (cVar1 != '\0');
    FUN_005e1a10(local_1d0,(int)pcVar17 - (int)(local_1d0 + 1));
    iVar9 = *(int *)(in_ECX + 200);
    if ((*(char *)(iVar9 + 0x1b0) != '\0') && (*(int *)(iVar9 + 0x1a8) != 0)) {
      FUN_005e1a10(*(int *)(iVar9 + 0x1a8),*(undefined4 *)(iVar9 + 0x1ac));
    }
  }
  else {
    pcVar17 = "send() HEAD";
    if (param_1 == 0) {
      pcVar17 = "send() GET";
    }
    iVar9 = send(*(SOCKET *)(in_ECX + 0x3200),local_1d0,*(int *)(local_1d0 + -0xc),0);
    FUN_00566020(iVar9,pcVar17);
    iVar9 = *(int *)(in_ECX + 200);
    if ((*(char *)(iVar9 + 0x1b0) != '\0') && (*(char **)(iVar9 + 0x1a8) != (char *)0x0)) {
      send(*(SOCKET *)(in_ECX + 0x3200),*(char **)(iVar9 + 0x1a8),*(int *)(iVar9 + 0x1ac),0);
    }
  }
  local_180._0_1_ = 0x11;
  FUN_00402570();
  local_180._0_1_ = 0x10;
  FUN_00402570();
  local_180._0_1_ = 0xd;
  FUN_00402570();
  local_180._0_1_ = 0xc;
  FUN_00402570();
  local_180._0_1_ = 0xb;
  FUN_00402570();
  local_180._0_1_ = 10;
  FUN_00402570();
  local_180._0_1_ = 9;
  FUN_00402570();
  local_180._0_1_ = 8;
  FUN_00402570();
  local_180._0_1_ = 7;
  FUN_00402570();
  local_180._0_1_ = 1;
  FUN_00402570();
  local_180 = (uint)local_180._1_3_ << 8;
  FUN_00402570();
  FUN_0058a843();
  return;
}


