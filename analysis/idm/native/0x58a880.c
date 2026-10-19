/* REA parent ev_6a698bc609ab4404ddaa417540036e53334d5c3c707a261178074fa8829e521f; decompiler output, not source. */

/* WARNING: Function: __alloca_probe replaced with injection: alloca_probe */

void FUN_0058a880(char param_1)

{
  char cVar1;
  wchar_t wVar2;
  undefined4 *puVar3;
  int iVar4;
  int iVar5;
  char *pcVar6;
  void *pvVar7;
  size_t sVar8;
  undefined4 uVar9;
  wchar_t *pwVar10;
  wchar_t *pwVar11;
  errno_t eVar12;
  LSTATUS LVar13;
  undefined2 *puVar14;
  int in_ECX;
  wchar_t *pwVar15;
  char *pcVar16;
  char *pcVar17;
  bool bVar18;
  uint local_5ac0;
  char *local_5a70;
  char *local_5a68;
  char *local_5a64;
  char *local_5a60;
  undefined1 *local_5a5c;
  void *local_5a58;
  undefined1 *puStack_5a54;
  uint local_5a50;
  undefined2 *local_5a4c;
  undefined2 local_5a48 [2];
  undefined4 local_5a44;
  wchar_t *local_5a40;
  wchar_t local_5a3c [2];
  undefined4 local_5a38;
  wchar_t *local_5a34;
  wchar_t local_5a30 [2];
  undefined4 local_5a2c;
  wchar_t *local_5a28;
  wchar_t local_5a24 [4];
  wchar_t *local_5a1c;
  wchar_t local_5a18 [4];
  wchar_t local_5a10 [10240];
  wchar_t local_a10 [256];
  wchar_t local_810 [256];
  wchar_t local_610 [256];
  wchar_t local_410 [256];
  wchar_t local_210;
  undefined1 local_20e [510];
  char *local_10 [2];
  uint local_8;
  
  puStack_5a54 = &LAB_006677c4;
  local_5a58 = ExceptionList;
  local_5ac0 = DAT_007608e0 ^ (uint)&local_5a4c;
  local_5a5c = (undefined1 *)&local_5ac0;
  ExceptionList = &local_5a58;
  local_5a50 = 0;
  local_8 = local_5ac0;
  FUN_004030b0(*(undefined4 *)(in_ECX + 0x3244));
  iVar5 = *(int *)(in_ECX + 0x3244);
  iVar4 = FUN_0057ecc0(iVar5);
  iVar5 = __memicmp((void *)(iVar5 + iVar4),"content-length:",0xf);
  if (iVar5 == 0) {
    if (param_1 == '\0') {
      FID_conflict__sscanf
                ((char *)(iVar4 + 0xf + *(int *)(in_ECX + 0x3244)),"%I64d",in_ECX + 0x3278);
    }
    else {
      FID_conflict__sscanf
                ((char *)(iVar4 + 0xf + *(int *)(in_ECX + 0x3244)),"%I64d",(int *)(in_ECX + 0x3218))
      ;
      if ((*(int *)(in_ECX + 0x3218) == 0) && (*(int *)(in_ECX + 0x321c) == 0)) {
        for (pcVar16 = (char *)(iVar4 + 0xf + *(int *)(in_ECX + 0x3244)); *pcVar16 == ' ';
            pcVar16 = pcVar16 + 1) {
        }
        if ((*pcVar16 == '0') && (*(int *)(in_ECX + 0x3208) != 0)) {
          *(undefined1 *)(*(int *)(in_ECX + 0x3208) + 0x889) = 1;
        }
        if ((-1 < *(int *)(in_ECX + 0x5c)) &&
           ((0 < *(int *)(in_ECX + 0x5c) || (*(int *)(in_ECX + 0x58) != 0)))) {
          iVar5 = *(int *)(in_ECX + 100);
          if ((-1 < iVar5) &&
             ((((0 < iVar5 || (*(int *)(in_ECX + 0x60) != 0)) &&
               (*(int *)(in_ECX + 0x60) == *(int *)(in_ECX + 0x78))) &&
              ((iVar5 == *(int *)(in_ECX + 0x7c) && (*(int *)(in_ECX + 0x3208) != 0)))))) {
            *(undefined1 *)(*(int *)(in_ECX + 0x3208) + 0x25f) = 1;
          }
        }
      }
      if ((*(int *)(in_ECX + 0x321c) < 1) && (*(int *)(in_ECX + 0x321c) < 0)) {
        *(undefined4 *)(in_ECX + 0x3218) = 0;
        *(undefined4 *)(in_ECX + 0x321c) = 0;
      }
    }
    goto LAB_0058a9e8;
  }
  if ((((*(int *)(in_ECX + 0x31fc) == 0) && (param_1 != '\0')) &&
      (iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"accept-ranges:",0xe),
      iVar5 == 0)) && (*(int *)(in_ECX + 0xc4) == 0)) {
    iVar5 = iVar4 + 0xe + *(int *)(in_ECX + 0x3244);
    iVar4 = FUN_0057ecc0(iVar5);
    pcVar16 = _strstr((char *)(iVar5 + iVar4),"none");
    if (pcVar16 == (char *)0x0) {
      *(undefined4 *)(in_ECX + 0x31fc) = 1;
    }
    else {
      if (*(int **)(in_ECX + 0x3208) != (int *)0x0) {
        (**(code **)(**(int **)(in_ECX + 0x3208) + 0xc))(2);
      }
      *(undefined4 *)(in_ECX + 0x31fc) = 2;
    }
    goto LAB_0058a9e8;
  }
  iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Content-Range:",0xe);
  if ((iVar5 == 0) && (param_1 != '\0')) {
    pcVar16 = _strchr((char *)(iVar4 + 0xe + *(int *)(in_ECX + 0x3244)),0x2f);
    if (pcVar16 != (char *)0x0) {
      cVar1 = pcVar16[1];
      if ((cVar1 != '\r') && (cVar1 != '\n')) {
        FID_conflict__sscanf(pcVar16 + 1,"%I64d",in_ECX + 0x3280);
      }
    }
    pcVar16 = _strchr((char *)(iVar4 + 0xe + *(int *)(in_ECX + 0x3244)),0x2d);
    if (pcVar16 != (char *)0x0) {
      cVar1 = pcVar16[1];
      if ((cVar1 != '\r') && (cVar1 != '\n')) {
        iVar5 = FID_conflict__sscanf(pcVar16 + 1,"%I64d",(undefined4 *)(in_ECX + 0x3288));
        if (iVar5 < 1) {
          *(undefined4 *)(in_ECX + 0x3288) = 0;
          *(undefined4 *)(in_ECX + 0x328c) = 0;
        }
      }
    }
    goto LAB_0058a9e8;
  }
  iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Transfer-Encoding:",0x12);
  if (iVar5 == 0) {
    pcVar16 = (char *)(iVar4 + 0x12 + *(int *)(in_ECX + 0x3244));
    while (pcVar16 != (char *)0x0) {
      for (; *pcVar16 == ' '; pcVar16 = pcVar16 + 1) {
      }
      iVar5 = __memicmp(pcVar16,"chunked",7);
      if (iVar5 == 0) {
        *(undefined1 *)(in_ECX + 0xe0) = 1;
        break;
      }
      pcVar16 = _strchr(pcVar16,0x2c);
      if (pcVar16 != (char *)0x0) {
        pcVar16 = pcVar16 + 1;
      }
    }
    goto LAB_0058a9e8;
  }
  iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Server: BestHop 2.4.4",0x15);
  if (((iVar5 == 0) && (param_1 != '\0')) ||
     ((iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Server: MediaFire-HTTP-lrbd",
                         0x1b), iVar5 == 0 && (param_1 != '\0')))) {
    if (*(int *)(in_ECX + 0x3208) != 0) {
      *(undefined1 *)(*(int *)(in_ECX + 0x3208) + 0x25f) = 1;
    }
    goto LAB_0058a9e8;
  }
  iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Content-Encoding:",0x11);
  if ((iVar5 == 0) && (param_1 != '\0')) {
    for (pcVar16 = (char *)(iVar4 + 0x11 + *(int *)(in_ECX + 0x3244)); *pcVar16 == ' ';
        pcVar16 = pcVar16 + 1) {
    }
    iVar5 = __memicmp(pcVar16,&DAT_006b5ed4,4);
    if ((((iVar5 == 0) || (iVar5 = __memicmp(pcVar16,"x-gzip",6), iVar5 == 0)) ||
        (iVar5 = __memicmp(pcVar16,"deflate",7), iVar5 == 0)) ||
       (iVar5 = __memicmp(pcVar16,"x-deflate",9), iVar5 == 0)) {
      *(undefined1 *)(in_ECX + 0x21) = 1;
      *(undefined1 *)(*(int *)(in_ECX + 200) + 0x35b) = 1;
    }
    iVar5 = __memicmp(pcVar16,&DAT_006b5eb4,3);
    if (((iVar5 == 0) || (iVar5 = __memicmp(pcVar16,&DAT_006b5eb0,3), iVar5 == 0)) ||
       (iVar5 = __memicmp(pcVar16,"brotli",6), iVar5 == 0)) {
      *(undefined1 *)(in_ECX + 0x20) = 1;
      *(undefined1 *)(in_ECX + 0x326d) = 1;
    }
    goto LAB_0058a9e8;
  }
  iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"WWW-Authenticate:",0x11);
  if (iVar5 == 0) {
    for (pcVar16 = (char *)(iVar4 + 0x11 + *(int *)(in_ECX + 0x3244));
        (*pcVar16 == ' ' || (pcVar17 = pcVar16, *pcVar16 == '\t')); pcVar16 = pcVar16 + 1) {
    }
    do {
      iVar5 = __memicmp(pcVar17,"realm",5);
      if (iVar5 == 0) {
        pcVar17 = pcVar17 + 5;
        goto LAB_0058ad80;
      }
      pcVar6 = _strchr(pcVar17,0x20);
    } while (((pcVar6 != (char *)0x0) || (pcVar6 = _strchr(pcVar17,9), pcVar6 != (char *)0x0)) &&
            (pcVar17 = pcVar6 + 1, *pcVar17 != '\0'));
LAB_0058ae4c:
    iVar5 = __memicmp(pcVar16,"Basic",5);
    if (iVar5 != 0) {
      *(undefined1 *)(*(int *)(in_ECX + 200) + 0x1b6) = 0;
      iVar5 = __memicmp(pcVar16,"Digest",6);
      if ((iVar5 == 0) && (pcVar17 = _strstr(pcVar16,"NTLM"), pcVar17 == (char *)0x0)) {
        pcVar17 = _strstr(pcVar16,"\r\n");
        if (((pcVar17 != (char *)0x0) || (pcVar17 = _strstr(pcVar16,"\n"), pcVar17 != (char *)0x0))
           && (sVar8 = (int)pcVar17 - (int)pcVar16, sVar8 != 0)) {
          *(undefined1 *)(*(int *)(in_ECX + 200) + 0x1b7) = 1;
          FUN_00402500();
          local_5a50._0_1_ = 1;
          pvVar7 = (void *)FUN_00402850(sVar8 + 1);
          _memcpy(pvVar7,pcVar16,sVar8);
          *(undefined1 *)((int)pvVar7 + sVar8) = 0;
          FUN_005ab190(local_5a60,"nonce",*(int *)(in_ECX + 200) + 0x2f4);
          FUN_005ab190(local_5a60,"realm",*(int *)(in_ECX + 200) + 0x2fc);
          FUN_005ab190(local_5a60,&DAT_006b6b04,*(int *)(in_ECX + 200) + 0x304);
          FUN_005ab190(local_5a60,"algorithm",*(int *)(in_ECX + 200) + 0x300);
          FUN_005ab190(local_5a60,"opaque",*(int *)(in_ECX + 200) + 0x308);
          pcVar16 = _strstr(local_5a60,"stale");
          if ((pcVar16 != (char *)0x0) &&
             (pcVar16 = _strchr(pcVar16 + 5,0x3d), pcVar16 != (char *)0x0)) {
            pcVar16 = pcVar16 + 1;
            FUN_00402500();
            local_5a50 = CONCAT31(local_5a50._1_3_,2);
            pcVar17 = _strchr(pcVar16,0x2c);
            if (pcVar17 == (char *)0x0) {
              FUN_00402fd0(pcVar16);
            }
            else {
              sVar8 = (int)pcVar17 - (int)pcVar16;
              local_5a64 = (char *)FUN_00402850(sVar8 + 1);
              _memcpy(local_5a64,pcVar16,sVar8);
              local_5a64[sVar8] = '\0';
            }
            iVar5 = __stricmp(local_10[0],"true");
            if (iVar5 == 0) {
              *(undefined1 *)(in_ECX + 0x32c4) = 0;
            }
            local_5a50._0_1_ = 1;
            FUN_00402570();
          }
          *(undefined4 *)(*(int *)(in_ECX + 200) + 0x30c) = 1;
          __time32((__time32_t *)&local_5a68);
          FUN_00402f70(*(int *)(in_ECX + 200) + 0x2f8,&DAT_0068397c,local_5a68);
          local_5a50 = (uint)local_5a50._1_3_ << 8;
          FUN_00402570();
        }
      }
      else if ((*(int *)(in_ECX + 0x32d4) == 0) && (*(int *)(in_ECX + 0x32d8) == 0)) {
        sVar8 = 0;
        pcVar17 = _strstr(pcVar16,"\r\n");
        if (((((pcVar17 != (char *)0x0) || (pcVar17 = _strstr(pcVar16,"\n"), pcVar17 != (char *)0x0)
              ) && (sVar8 = (int)pcVar17 - (int)pcVar16, 6 < (int)sVar8)) &&
            ((iVar5 = __memicmp(pcVar16,"Bearer ",7), iVar5 == 0 &&
             (pcVar17 = _strstr(pcVar16,"NTLM"), pcVar17 == (char *)0x0)))) &&
           (pcVar17 = _strstr(pcVar16,"Digest"), pcVar17 == (char *)0x0)) {
          sVar8 = 6;
        }
        _memcpy(*(void **)(*(int *)(in_ECX + 200) + 0x2f0),pcVar16,sVar8);
        *(undefined1 *)(sVar8 + *(int *)(*(int *)(in_ECX + 200) + 0x2f0)) = 0;
      }
      else {
        pcVar16 = _strchr(pcVar16,0x20);
        pcVar17 = (char *)0x0;
        local_10[0] = (char *)0x0;
        local_5a68 = pcVar16;
        if (pcVar16 != (char *)0x0) {
          pcVar16 = pcVar16 + 1;
          local_5a68 = pcVar16;
          pcVar6 = _strstr(pcVar16,"\r\n");
          if ((pcVar6 != (char *)0x0) || (pcVar6 = _strstr(pcVar16,"\n"), pcVar6 != (char *)0x0)) {
            pcVar17 = pcVar6 + -(int)pcVar16;
            local_10[0] = pcVar17;
          }
        }
        if (*(int *)(in_ECX + 0x32cc) != 0) {
          local_5a50 = CONCAT31(local_5a50._1_3_,3);
          FUN_005f93e2(*(undefined4 *)(in_ECX + 0x32cc));
          local_5a50 = 0;
          FUN_0058b1bc();
          return;
        }
        if ((pcVar17 == (char *)0x0) && (*(int *)(in_ECX + 0xa0) != 0)) {
          if (*(int *)(in_ECX + 0x32d4) != 0) {
            FUN_0057f460(in_ECX + 0x3314);
            *(undefined4 *)(in_ECX + 0x32d4) = 0;
          }
          *(undefined4 *)(in_ECX + 0x32d4) = 0;
          *(undefined4 *)(in_ECX + 0x32d8) = 0;
        }
        else {
          pvVar7 = (void *)FUN_005f93d7(pcVar17 + 1);
          *(void **)(in_ECX + 0x32cc) = pvVar7;
          _memcpy(pvVar7,pcVar16,(size_t)pcVar17);
          pcVar17[*(int *)(in_ECX + 0x32cc)] = '\0';
        }
      }
    }
    goto LAB_0058a9e8;
  }
  iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Proxy-Authenticate:",0x13);
  if (iVar5 == 0) {
    for (pcVar16 = (char *)(iVar4 + 0x13 + *(int *)(in_ECX + 0x3244));
        (*pcVar16 == ' ' || (*pcVar16 == '\t')); pcVar16 = pcVar16 + 1) {
    }
    iVar5 = __memicmp(pcVar16,"Basic",5);
    if (iVar5 != 0) {
      iVar5 = __memicmp(pcVar16,"Digest",6);
      if (((iVar5 == 0) && (pcVar17 = _strstr(pcVar16,"NTLM"), pcVar17 == (char *)0x0)) &&
         (*(int *)(*(int *)(in_ECX + 0x19c) + 8) != 2)) {
        *(undefined4 *)(*(int *)(in_ECX + 0x19c) + 8) = 3;
        pcVar17 = _strstr(pcVar16,"\r\n");
        if (((pcVar17 != (char *)0x0) || (pcVar17 = _strstr(pcVar16,"\n"), pcVar17 != (char *)0x0))
           && (sVar8 = (int)pcVar17 - (int)pcVar16, sVar8 != 0)) {
          FUN_00402500();
          local_5a50._0_1_ = 5;
          local_5a64 = (char *)FUN_00402850(sVar8 + 1);
          _memcpy(local_5a64,pcVar16,sVar8);
          local_5a64[sVar8] = '\0';
          FUN_00402500();
          local_5a50._0_1_ = 6;
          uVar9 = FUN_0057dd60((*(undefined4 **)(in_ECX + 200))[9],**(undefined4 **)(in_ECX + 200));
          FUN_00402f70(local_10,"%s/%s",uVar9);
          FUN_005ab230(local_5a60,local_10[0]);
          local_5a50._0_1_ = 5;
          FUN_00402570();
          local_5a50 = (uint)local_5a50._1_3_ << 8;
          FUN_00402570();
        }
      }
      else {
        *(undefined4 *)(*(int *)(in_ECX + 0x19c) + 8) = 2;
        if ((*(int *)(in_ECX + 0x32e0) == 0) && (*(int *)(in_ECX + 0x32e4) == 0)) {
          iVar5 = __stricmp(*(char **)(*(int *)(in_ECX + 0x19c) + 0x10c),"NTLM");
          if (iVar5 != 0) {
            sVar8 = 0;
            pcVar17 = _strstr(pcVar16,"\r\n");
            if ((pcVar17 != (char *)0x0) ||
               (pcVar17 = _strstr(pcVar16,"\n"), pcVar17 != (char *)0x0)) {
              sVar8 = (int)pcVar17 - (int)pcVar16;
            }
            _memcpy(*(void **)(*(int *)(in_ECX + 0x19c) + 0x10c),pcVar16,sVar8);
            *(undefined1 *)(sVar8 + *(int *)(*(int *)(in_ECX + 0x19c) + 0x10c)) = 0;
          }
        }
        else {
          pcVar16 = _strchr(pcVar16,0x20);
          pcVar17 = (char *)0x0;
          local_10[0] = (char *)0x0;
          local_5a68 = pcVar16;
          if (pcVar16 != (char *)0x0) {
            pcVar16 = pcVar16 + 1;
            local_5a68 = pcVar16;
            pcVar6 = _strstr(pcVar16,"\r\n");
            if ((pcVar6 != (char *)0x0) || (pcVar6 = _strstr(pcVar16,"\n"), pcVar6 != (char *)0x0))
            {
              pcVar17 = pcVar6 + -(int)pcVar16;
              local_10[0] = pcVar17;
            }
          }
          if (*(int *)(in_ECX + 0x32e8) != 0) {
            local_5a50 = CONCAT31(local_5a50._1_3_,7);
            FUN_005f93e2(*(undefined4 *)(in_ECX + 0x32e8));
            local_5a50 = 0;
            FUN_0058b4ca();
            return;
          }
          if ((pcVar17 == (char *)0x0) && (*(int *)(in_ECX + 0xa0) != 0)) {
            if (*(int *)(in_ECX + 0x32e0) != 0) {
              FUN_0057f460(in_ECX + 0x32f0);
              *(undefined4 *)(in_ECX + 0x32e0) = 0;
            }
            *(undefined4 *)(in_ECX + 0x32e0) = 0;
            *(undefined4 *)(in_ECX + 0x32e4) = 0;
          }
          else {
            pvVar7 = (void *)FUN_005f93d7(pcVar17 + 1);
            *(void **)(in_ECX + 0x32e8) = pvVar7;
            _memcpy(pvVar7,pcVar16,(size_t)pcVar17);
            pcVar17[*(int *)(in_ECX + 0x32e8)] = '\0';
          }
        }
      }
    }
    goto LAB_0058a9e8;
  }
  iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Location:",9);
  if ((iVar5 == 0) && (param_1 != '\0')) {
    if (*(int *)(in_ECX + 0x31f8) != 0) {
      FUN_005f93e2(*(int *)(in_ECX + 0x31f8));
    }
    iVar5 = iVar4 + 9 + *(int *)(in_ECX + 0x3244);
    iVar4 = FUN_0057ecc0(iVar5);
    pcVar17 = (char *)(iVar5 + iVar4);
    sVar8 = 0;
    pcVar16 = _strstr(pcVar17,"\r\n");
    if ((pcVar16 != (char *)0x0) || (pcVar16 = _strstr(pcVar17,"\n"), pcVar16 != (char *)0x0)) {
      sVar8 = (int)pcVar16 - (int)pcVar17;
    }
    pvVar7 = (void *)FUN_005f93d7(sVar8 + 1);
    *(void **)(in_ECX + 0x31f8) = pvVar7;
    _memcpy(pvVar7,pcVar17,sVar8);
    *(undefined1 *)(sVar8 + *(int *)(in_ECX + 0x31f8)) = 0;
    goto LAB_0058a9e8;
  }
  iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Content-Disposition:",0x14);
  if ((iVar5 != 0) || (param_1 == '\0')) {
    iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Content-Type:",0xd);
    if ((iVar5 == 0) && (param_1 != '\0')) {
      iVar5 = *(int *)(in_ECX + 0x325c);
      if (((iVar5 == 200) || (iVar5 == 0xce)) || (iVar5 == 0x130)) {
        iVar5 = *(int *)(*(int *)(in_ECX + 200) + 0xbc);
        if (iVar5 != 0) {
          FUN_005f93e2(iVar5);
        }
        iVar5 = iVar4 + 0xd + *(int *)(in_ECX + 0x3244);
        iVar4 = FUN_0057ecc0(iVar5);
        pcVar17 = (char *)(iVar5 + iVar4);
        local_5a68 = (char *)0x0;
        pcVar16 = _strstr(pcVar17,"\r\n");
        local_10[0] = _strchr(pcVar17,0x3b);
        if ((((local_10[0] < pcVar16) && (local_10[0] != (char *)0x0)) || (pcVar16 == (char *)0x0))
           && (pcVar16 = local_10[0], local_10[0] == (char *)0x0)) {
          pcVar6 = _strstr(pcVar17,"\n");
          pcVar16 = local_5a68;
          if (pcVar6 != (char *)0x0) {
            pcVar16 = pcVar6 + -(int)pcVar17;
          }
        }
        else {
          local_5a68 = pcVar16 + -(int)pcVar17;
          pcVar16 = local_5a68;
        }
        uVar9 = FUN_005f93d7(pcVar16 + 5);
        *(undefined4 *)(*(int *)(in_ECX + 200) + 0xbc) = uVar9;
        _memcpy(*(void **)(*(int *)(in_ECX + 200) + 0xbc),pcVar17,(size_t)pcVar16);
        pcVar16[*(int *)(*(int *)(in_ECX + 200) + 0xbc)] = '\0';
        if ((*(char *)(*(int *)(in_ECX + 200) + 0x179) == '\0') ||
           (iVar5 = __memicmp(*(void **)(*(int *)(in_ECX + 200) + 0xbc),"text/html",9), iVar5 != 0))
        {
          pcVar17 = *(char **)(*(int *)(in_ECX + 200) + 0xc4);
          pcVar16 = pcVar17 + 1;
          do {
            cVar1 = *pcVar17;
            pcVar17 = pcVar17 + 1;
          } while (cVar1 != '\0');
          if ((pcVar17 == pcVar16) ||
             (iVar5 = __memicmp(*(void **)(*(int *)(in_ECX + 200) + 0xbc),"text/html",9), iVar5 != 0
             )) {
            iVar5 = __stricmp(*(char **)(*(int *)(in_ECX + 200) + 0xbc),
                              "application/vnd.lumberjack.manifest");
            if ((iVar5 == 0) && ((*(uint *)(*(int *)(in_ECX + 200) + 0x164) & 0x100) != 0)) {
              FUN_005f93e2(*(undefined4 *)(*(int *)(in_ECX + 200) + 0xbc));
              *(undefined4 *)(*(int *)(in_ECX + 200) + 0xbc) = 0;
              *(undefined4 *)(*(int *)(in_ECX + 200) + 0x340) = 1;
              *(undefined1 *)(*(int *)(in_ECX + 200) + 0x344) = 1;
            }
            else {
              iVar5 = __memicmp(*(void **)(*(int *)(in_ECX + 200) + 0xbc),"multipart/byteranges",
                                0x14);
              if ((((iVar5 == 0) ||
                   (iVar5 = __memicmp(*(void **)(*(int *)(in_ECX + 200) + 0xbc),
                                      "multipart/x-byteranges",0x16), iVar5 == 0)) &&
                  (local_10[0] != (char *)0x0)) &&
                 ((pcVar16 = _strstr(local_10[0],"boundary"), pcVar16 != (char *)0x0 &&
                  (pcVar16 = _strchr(pcVar16,0x3d), pcVar16 != (char *)0x0)))) {
                iVar5 = FUN_0057ecc0(pcVar16 + 1);
                pcVar17 = pcVar16 + 1 + iVar5;
                sVar8 = 0;
                pcVar16 = _strstr(pcVar17,"\r\n");
                if ((pcVar16 != (char *)0x0) ||
                   (pcVar16 = _strstr(pcVar17,"\n"), pcVar16 != (char *)0x0)) {
                  sVar8 = (int)pcVar16 - (int)pcVar17;
                }
                pvVar7 = (void *)FUN_00402850(sVar8);
                _memcpy(pvVar7,pcVar17,sVar8);
                pcVar17 = *(char **)(in_ECX + 0x3268);
                pcVar16 = pcVar17 + 1;
                do {
                  cVar1 = *pcVar17;
                  pcVar17 = pcVar17 + 1;
                } while (cVar1 != '\0');
                if (pcVar17 != pcVar16) {
                  *(undefined1 *)(in_ECX + 0x326c) = 1;
                }
              }
            }
            goto LAB_0058a9e8;
          }
        }
        FUN_005f93e2(*(undefined4 *)(*(int *)(in_ECX + 200) + 0xbc));
        *(undefined4 *)(*(int *)(in_ECX + 200) + 0xbc) = 0;
      }
    }
    else {
      iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Set-Cookie:",0xb);
      if ((iVar5 == 0) && (param_1 != '\0')) {
        FUN_00585e80(iVar4 + 0xb + *(int *)(in_ECX + 0x3244));
      }
      else {
        iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"ETag:",5);
        if (iVar5 == 0) {
          iVar5 = iVar4 + 5 + *(int *)(in_ECX + 0x3244);
          iVar4 = FUN_0057ecc0(iVar5);
          pcVar17 = (char *)(iVar5 + iVar4);
          sVar8 = 0;
          pcVar16 = _strstr(pcVar17,"\r\n");
          if ((pcVar16 != (char *)0x0) || (pcVar16 = _strstr(pcVar17,"\n"), pcVar16 != (char *)0x0))
          {
            sVar8 = (int)pcVar16 - (int)pcVar17;
          }
          pvVar7 = (void *)FUN_00402850(sVar8);
          _memcpy(pvVar7,pcVar17,sVar8);
        }
        else {
          iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),&DAT_006b69e8,4);
          if (iVar5 == 0) {
            iVar5 = iVar4 + 4 + *(int *)(in_ECX + 0x3244);
            iVar4 = FUN_0057ecc0(iVar5);
            iVar5 = __memicmp((void *)(iVar5 + iVar4),"policyref=\"http://p3p.yahoo.com/w3c/",0x23);
            if (iVar5 == 0) {
              *(undefined1 *)(in_ECX + 0x326e) = 1;
            }
          }
          else {
            iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Date:",5);
            if ((iVar5 == 0) && (param_1 != '\0')) {
              iVar5 = iVar4 + 5 + *(int *)(in_ECX + 0x3244);
              iVar4 = FUN_0057ecc0(iVar5);
              pcVar17 = (char *)(iVar5 + iVar4);
              sVar8 = 0;
              pcVar16 = _strstr(pcVar17,"\r\n");
              if ((pcVar16 != (char *)0x0) ||
                 (pcVar16 = _strstr(pcVar17,"\n"), pcVar16 != (char *)0x0)) {
                sVar8 = (int)pcVar16 - (int)pcVar17;
              }
              local_5a68 = (char *)FUN_005f93d7(sVar8 + 1);
              _memcpy(local_5a68,pcVar17,sVar8);
              pcVar16 = local_5a68;
              local_5a68[sVar8] = '\0';
              FUN_00592a10();
              local_5a50._0_1_ = 0x1c;
              uVar9 = FUN_00593950(pcVar16);
              *(undefined4 *)(*(int *)(in_ECX + 200) + 0x3b4) = uVar9;
              FUN_005f93e2(pcVar16);
              local_5a50 = (uint)local_5a50._1_3_ << 8;
              FUN_00592a20();
            }
            else {
              iVar5 = __memicmp((void *)(*(int *)(in_ECX + 0x3244) + iVar4),"Last-Modified:",0xe);
              if ((iVar5 == 0) && ((*(int *)(in_ECX + 0xc4) == 0 && (param_1 != '\0')))) {
                iVar5 = iVar4 + 0xe + *(int *)(in_ECX + 0x3244);
                iVar4 = FUN_0057ecc0(iVar5);
                pcVar17 = (char *)(iVar5 + iVar4);
                sVar8 = 0;
                pcVar16 = _strstr(pcVar17,"\r\n");
                if ((pcVar16 != (char *)0x0) ||
                   (pcVar16 = _strstr(pcVar17,"\n"), pcVar16 != (char *)0x0)) {
                  sVar8 = (int)pcVar16 - (int)pcVar17;
                }
                local_5a68 = (char *)FUN_005f93d7(sVar8 + 1);
                _memcpy(local_5a68,pcVar17,sVar8);
                pcVar16 = local_5a68;
                local_5a68[sVar8] = '\0';
                pvVar7 = *(void **)(*(int *)(in_ECX + 200) + 0x20);
                if (pvVar7 == (void *)0x0) {
                  FUN_005d62d0(local_5a68,*(undefined4 *)(*(int *)(in_ECX + 0x3208) + 0xb8));
                }
                else {
                  iVar5 = __memicmp(local_5a68,pvVar7,sVar8);
                  if ((iVar5 != 0) && (*(int *)(*(int *)(in_ECX + 0x3208) + 0x144) == 0)) {
                    FUN_00402f70(in_ECX + 0x322c,&DAT_0067def0,pcVar16);
                  }
                }
                FUN_00592a10();
                local_5a50._0_1_ = 0x1d;
                uVar9 = FUN_00593950(pcVar16);
                *(undefined4 *)(*(int *)(in_ECX + 200) + 0x3b8) = uVar9;
                FUN_005f93e2(pcVar16);
                local_5a50 = (uint)local_5a50._1_3_ << 8;
                FUN_00592a20();
              }
              local_5a50 = 0xffffffff;
            }
          }
        }
      }
    }
    goto LAB_0058a9e8;
  }
  if (((*(int *)(in_ECX + 0x325c) != 200) && (*(int *)(in_ECX + 0x325c) != 0xce)) ||
     (*(int *)(*(int *)(in_ECX + 200) + 0x340) != 0)) goto LAB_0058a9e8;
  bVar18 = false;
  pcVar16 = _strstr((char *)(iVar4 + 0x14 + *(int *)(in_ECX + 0x3244)),"attachment");
  if (pcVar16 == (char *)0x0) {
    pcVar16 = _strstr((char *)(iVar4 + 0x14 + *(int *)(in_ECX + 0x3244)),"inline;");
    if (pcVar16 != (char *)0x0) {
      *(undefined1 *)(*(int *)(in_ECX + 200) + 0x196) = 1;
    }
  }
  else {
    bVar18 = true;
  }
  *(int *)(in_ECX + 0x32ac) = *(int *)(in_ECX + 0x32ac) + 1;
  FUN_0057f9b0(iVar4 + 0x14 + *(int *)(in_ECX + 0x3244));
  pcVar16 = _strstr((char *)(iVar4 + 0x14 + *(int *)(in_ECX + 0x3244)),"filename");
  if (pcVar16 == (char *)0x0) {
    pcVar16 = (char *)(iVar4 + 0x14 + *(int *)(in_ECX + 0x3244));
    do {
      pcVar16 = _strpbrk(pcVar16,"Ff");
      while( true ) {
        if (pcVar16 == (char *)0x0) {
          if (bVar18) {
            FUN_005845c0();
          }
          goto LAB_0058a9e8;
        }
        iVar5 = __memicmp(pcVar16,"filename",8);
        if (iVar5 == 0) goto LAB_0058b712;
        pcVar16 = pcVar16 + 2;
        if (*pcVar16 != '\0') break;
        pcVar16 = (char *)0x0;
      }
    } while( true );
  }
LAB_0058b712:
  pcVar16 = _strchr(pcVar16,0x3d);
  if (pcVar16 == (char *)0x0) goto LAB_0058a9e8;
  pcVar16 = pcVar16 + 1;
  if ((*pcVar16 == '\r') || (*pcVar16 == '\n')) goto LAB_0058a9e8;
  iVar5 = FUN_0057ecc0(pcVar16);
  pcVar16 = pcVar16 + iVar5;
  if (*pcVar16 == '\0') goto LAB_0058a9e8;
  bVar18 = true;
  if (*pcVar16 == '\"') {
    iVar5 = 0x22;
LAB_0058b75e:
    pcVar16 = pcVar16 + 1;
    local_5a70 = _strchr(pcVar16,iVar5);
    if (local_5a70 == (char *)0x0) goto LAB_0058b8fe;
    bVar18 = false;
LAB_0058b776:
    local_10[0] = local_5a70 + -(int)pcVar16;
  }
  else {
    if (*pcVar16 == '\'') {
      iVar5 = 0x27;
      goto LAB_0058b75e;
    }
LAB_0058b8fe:
    local_5a70 = _strstr(pcVar16,"\r\n");
    if ((local_5a70 != (char *)0x0) ||
       (local_5a70 = _strstr(pcVar16,"\n"), local_5a70 != (char *)0x0)) goto LAB_0058b776;
    local_10[0] = pcVar16;
    do {
      cVar1 = *local_10[0];
      local_10[0] = local_10[0] + 1;
    } while (cVar1 != '\0');
    local_10[0] = local_10[0] + -(int)(pcVar16 + 1);
    if ((local_10[0] == (char *)0x0) || (0x200 < (int)local_10[0])) goto LAB_0058a9e8;
  }
  FUN_00402500();
  local_5a50 = CONCAT31(local_5a50._1_3_,9);
  pcVar17 = (char *)FUN_00402850(local_10[0]);
  _memcpy(pcVar17,pcVar16,(size_t)local_10[0]);
  local_10[0][(int)pcVar17] = '\0';
  if (bVar18) {
    pcVar16 = _strchr(local_5a60,0x3b);
    if ((pcVar16 != (char *)0x0) ||
       ((pcVar16 = _strchr(local_5a60,0x2e), pcVar16 != (char *)0x0 &&
        (pcVar16 = _strchr(pcVar16 + 1,0x2c), pcVar16 != (char *)0x0)))) {
      *pcVar16 = '\0';
      iVar5 = __memicmp(local_5a60,"utf-8\'\'",7);
      if ((iVar5 != 0) &&
         ((pcVar16 = _strstr(pcVar16 + 1,"filename"), pcVar16 != (char *)0x0 &&
          (pcVar16 = _strchr(pcVar16 + 8,0x3d), pcVar16 != (char *)0x0)))) {
        iVar5 = __memicmp(pcVar16 + 1,"utf-8\'\'",7);
        if (iVar5 == 0) {
          FUN_00402500();
          local_5a50._0_1_ = 10;
          FUN_00402f70(local_10,&DAT_0067def0,pcVar16 + 1);
          pcVar16 = local_10[0];
          do {
            cVar1 = *pcVar16;
            pcVar16 = pcVar16 + 1;
          } while (cVar1 != '\0');
          sVar8 = (int)pcVar16 - (int)(local_10[0] + 1);
          pcVar17 = (char *)FUN_00402850(sVar8);
          _memcpy(pcVar17,local_10[0],sVar8);
          pcVar17[sVar8] = '\0';
          pcVar16 = _strchr(local_5a60,0x3b);
          if (pcVar16 != (char *)0x0) {
            *pcVar16 = '\0';
          }
          goto LAB_0058ba2f;
        }
      }
    }
  }
  else {
    pcVar16 = local_5a70 + 1;
    local_5a64 = pcVar16;
    if ((*pcVar16 == '.') &&
       (((local_5a70 = _strstr(pcVar16,"\r\n"), local_5a70 != (char *)0x0 ||
         (local_5a70 = _strstr(pcVar16,"\n"), local_5a70 != (char *)0x0)) &&
        (sVar8 = (int)local_5a70 - (int)local_5a64, sVar8 - 2 < 4)))) {
      FUN_00402500();
      local_5a50._0_1_ = 0xb;
      pvVar7 = (void *)FUN_00402850(sVar8);
      _memcpy(pvVar7,local_5a64,sVar8);
      *(undefined1 *)((int)pvVar7 + sVar8) = 0;
      FUN_00402500();
      local_5a50._0_1_ = 0xc;
      FUN_00402f70(local_10,&DAT_0067de30,local_5a60,local_5a68);
      FUN_00402f70(&local_5a60,&DAT_0067def0,local_10[0]);
      pcVar17 = local_5a60;
      local_5a50._0_1_ = 0xb;
      FUN_00402570();
LAB_0058ba2f:
      local_5a50 = CONCAT31(local_5a50._1_3_,9);
      FUN_00402570();
    }
  }
  iVar5 = __memicmp(pcVar17,"=?UTF-8?",8);
  if (iVar5 == 0) {
    FUN_005daa90(pcVar17,&local_5a60);
  }
  else {
    iVar5 = __memicmp(pcVar17,"=?UTF8?B?",9);
    if (iVar5 == 0) {
      FUN_005daa90(pcVar17,&local_5a60);
    }
    else {
      iVar5 = __memicmp(pcVar17,"ISO-8859\'\'",10);
      if (iVar5 == 0) {
        FUN_004030f0(local_5a60 + 10);
        local_5a50 = CONCAT31(local_5a50._1_3_,0xd);
        ATL::CSimpleStringT<char,0>::operator=
                  ((CSimpleStringT<char,0> *)&local_5a60,(CSimpleStringT<char,0> *)&local_5a64);
      }
      else {
        iVar5 = __memicmp(pcVar17,"ISO-8859-",9);
        if (iVar5 == 0) {
          FUN_004030f0(local_5a60 + 9);
          local_5a50 = CONCAT31(local_5a50._1_3_,0xe);
        }
        else {
          iVar5 = __memicmp(pcVar17,"windows-125",0xb);
          if (iVar5 != 0) goto LAB_0058bb43;
          FUN_004030f0(local_5a60 + 0xb);
          local_5a50 = CONCAT31(local_5a50._1_3_,0xf);
        }
        pcVar16 = _strstr(local_5a64,"\'\'");
        if ((pcVar16 != (char *)0x0) && (pcVar16[2] != '\0')) {
          FUN_00402fd0(pcVar16 + 2);
        }
      }
      local_5a50 = CONCAT31(local_5a50._1_3_,9);
      FUN_00402570();
    }
  }
LAB_0058bb43:
  iVar5 = __memicmp(local_5a60,"utf-8\'",6);
  if ((iVar5 != 0) && (local_5a70 != (char *)0x0)) {
    pcVar16 = _strstr(local_5a70,"filename");
    if (pcVar16 == (char *)0x0) {
      do {
        pcVar16 = _strpbrk(local_5a70,"Ff");
        while( true ) {
          if (pcVar16 == (char *)0x0) goto LAB_0058bd99;
          iVar5 = __memicmp(pcVar16,"filename",8);
          if (iVar5 == 0) goto LAB_0058bbc3;
          local_5a70 = pcVar16 + 2;
          if (*local_5a70 != '\0') break;
          pcVar16 = (char *)0x0;
        }
      } while( true );
    }
LAB_0058bbc3:
    pcVar16 = _strchr(pcVar16,0x3d);
    if (pcVar16 != (char *)0x0) {
      pcVar16 = pcVar16 + 1;
      if ((*pcVar16 != '\r') && (*pcVar16 != '\n')) {
        iVar5 = FUN_0057ecc0(pcVar16);
        pcVar17 = pcVar16 + iVar5;
        if (*pcVar17 != '\0') {
          bVar18 = true;
          if (*pcVar17 == '\"') {
            pcVar17 = pcVar17 + 1;
            local_5a68 = _strchr(pcVar17,0x22);
            if (local_5a68 == (char *)0x0) goto LAB_0058bcad;
            bVar18 = false;
LAB_0058bc32:
            local_5a68 = local_5a68 + -(int)pcVar17;
          }
          else {
LAB_0058bcad:
            local_5a68 = _strstr(pcVar17,"\r\n");
            if ((local_5a68 != (char *)0x0) ||
               (local_5a68 = _strstr(pcVar17,"\n"), local_5a68 != (char *)0x0)) goto LAB_0058bc32;
            local_5a68 = pcVar17;
            do {
              cVar1 = *local_5a68;
              local_5a68 = local_5a68 + 1;
            } while (cVar1 != '\0');
            local_5a68 = local_5a68 + -(int)(pcVar17 + 1);
            if ((local_5a68 == (char *)0x0) || (0x100 < (int)local_5a68)) {
              local_5a50 = local_5a50 & 0xffffff00;
              FUN_00402570();
              goto LAB_0058a9e8;
            }
          }
          pcVar16 = (char *)FUN_00402850(local_5a68);
          _memcpy(pcVar16,pcVar17,(size_t)local_5a68);
          if ((bVar18) && (pcVar17 = _strchr(local_5a60,0x3b), pcVar17 != (char *)0x0)) {
            *pcVar17 = '\0';
          }
        }
      }
    }
    iVar5 = __memicmp(pcVar16,"ISO-8859\'\'",10);
    if (iVar5 == 0) {
      FUN_004030f0(local_5a60 + 10);
      local_5a50 = CONCAT31(local_5a50._1_3_,0x10);
      ATL::CSimpleStringT<char,0>::operator=
                ((CSimpleStringT<char,0> *)&local_5a60,(CSimpleStringT<char,0> *)&local_5a64);
    }
    else {
      iVar5 = __memicmp(pcVar16,"ISO-8859-",9);
      if (iVar5 == 0) {
        FUN_004030f0(local_5a60 + 9);
        local_5a50 = CONCAT31(local_5a50._1_3_,0x11);
      }
      else {
        iVar5 = __memicmp(pcVar16,"windows-125",0xb);
        if (iVar5 != 0) goto LAB_0058bd99;
        FUN_004030f0(local_5a60 + 0xb);
        local_5a50 = CONCAT31(local_5a50._1_3_,0x12);
      }
      pcVar16 = _strstr(local_5a64,"\'\'");
      if ((pcVar16 != (char *)0x0) && (pcVar16[2] != '\0')) {
        FUN_00402fd0(pcVar16 + 2);
      }
    }
    local_5a50 = CONCAT31(local_5a50._1_3_,9);
    FUN_00402570();
  }
LAB_0058bd99:
  FUN_00407030(local_5a60);
  local_5a50 = CONCAT31(local_5a50._1_3_,0x13);
  ATL::CSimpleStringT<char,0>::operator=
            ((CSimpleStringT<char,0> *)(*(int *)(in_ECX + 200) + 0xc4),
             (CSimpleStringT<char,0> *)&local_5a60);
  iVar5 = __memicmp(local_5a60,"utf-8\'\'",7);
  if ((iVar5 == 0) || (iVar5 = __memicmp(local_5a60,"utf-8\' \'",8), iVar5 == 0)) {
    iVar5 = __memicmp(local_5a60,"utf-8\'\'",7);
    FUN_004030f0(local_5a60 + (iVar5 != 0) + 7);
    local_5a50._0_1_ = 0x14;
    FUN_00401980(local_5a68,0);
    ATL::CSimpleStringT<char,0>::operator=
              ((CSimpleStringT<char,0> *)(*(int *)(in_ECX + 200) + 0xc4),
               (CSimpleStringT<char,0> *)&local_5a68);
    *(undefined1 *)(*(int *)(in_ECX + 200) + 0x34d) = 1;
    local_5a50 = CONCAT31(local_5a50._1_3_,0x13);
    FUN_00402570();
  }
  FUN_005c27e0(*(undefined4 *)(*(int *)(in_ECX + 200) + 0xc4),
               -(uint)(*(char *)(*(int *)(in_ECX + 200) + 0x34d) != '\0') & 0xfde9,&local_5a1c,1,1,0
               ,1);
  pcVar16 = *(char **)(*(int *)(in_ECX + 200) + 0xbc);
  if (pcVar16 != (char *)0x0) {
    pcVar17 = pcVar16;
    do {
      cVar1 = *pcVar17;
      pcVar17 = pcVar17 + 1;
    } while (cVar1 != '\0');
    if ((pcVar17 != pcVar16 + 1) && (iVar5 = __memicmp(pcVar16,"text/html",9), iVar5 == 0)) {
      FUN_005f93e2(*(undefined4 *)(*(int *)(in_ECX + 200) + 0xbc));
      *(undefined4 *)(*(int *)(in_ECX + 200) + 0xbc) = 0;
    }
  }
  if ((((*(int *)(in_ECX + 0xc4) == 0) &&
       (iVar5 = *(int *)(in_ECX + 200), *(int *)(iVar5 + 0x340) == 0)) &&
      (*(int *)(iVar5 + 0x334) == 0)) &&
     ((*(char *)(iVar5 + 0x351) == '\0' &&
      (((*(uint *)(iVar5 + 0x164) & 0x100) == 0 || ((*(uint *)(iVar5 + 0x164) & 0x2000) == 0)))))) {
    pwVar10 = local_5a1c;
    if (local_5a1c == (wchar_t *)0x0) {
      pwVar10 = local_5a18;
    }
    pwVar10 = _wcschr(pwVar10,L'%');
    if (pwVar10 != (wchar_t *)0x0) {
      pwVar10 = local_5a1c;
      if (local_5a1c == (wchar_t *)0x0) {
        pwVar10 = local_5a18;
      }
      pwVar11 = pwVar10 + 1;
      do {
        wVar2 = *pwVar10;
        pwVar10 = pwVar10 + 1;
      } while (wVar2 != L'\0');
      if ((uint)((int)pwVar10 - (int)pwVar11 >> 1) < 0x2800) {
        pwVar10 = local_5a1c;
        if (local_5a1c == (wchar_t *)0x0) {
          pwVar10 = local_5a18;
        }
        pwVar11 = local_5a10;
        do {
          wVar2 = *pwVar10;
          *pwVar11 = wVar2;
          pwVar10 = pwVar10 + 1;
          pwVar11 = pwVar11 + 1;
        } while (wVar2 != L'\0');
        FUN_005bcde0(local_5a10,&local_5a1c);
      }
    }
    pwVar10 = local_5a1c;
    if (local_5a1c == (wchar_t *)0x0) {
      pwVar10 = local_5a18;
    }
    pwVar11 = pwVar10 + 1;
    do {
      wVar2 = *pwVar10;
      pwVar10 = pwVar10 + 1;
    } while (wVar2 != L'\0');
    if (0x100 < (uint)((int)pwVar10 - (int)pwVar11 >> 1)) {
      pwVar10 = local_5a1c;
      if (local_5a1c == (wchar_t *)0x0) {
        pwVar10 = local_5a18;
      }
      pwVar11 = _wcsrchr(pwVar10,L'.');
      pwVar10 = local_5a1c;
      if (local_5a1c == (wchar_t *)0x0) {
        pwVar10 = local_5a18;
      }
      if (pwVar11 != (wchar_t *)0x0) {
        pwVar15 = pwVar11;
        do {
          wVar2 = *pwVar15;
          pwVar15 = pwVar15 + 1;
        } while (wVar2 != L'\0');
        if ((uint)((int)pwVar15 - (int)(pwVar11 + 1) >> 1) < 10) {
          pwVar15 = pwVar11;
          do {
            wVar2 = *pwVar15;
            pwVar15 = pwVar15 + 1;
          } while (wVar2 != L'\0');
          pwVar10[0xfd - ((int)pwVar15 - (int)(pwVar11 + 1) >> 1)] = L'\0';
          local_5a4c = (undefined2 *)0x0;
          local_5a44 = 0;
          local_5a48[0] = 0;
          local_5a50._0_1_ = 0x15;
          FUN_00402320(&DAT_0067de30,pwVar10,pwVar11,0);
          puVar14 = local_5a4c;
          if (local_5a4c == (undefined2 *)0x0) {
            puVar14 = local_5a48;
          }
          FUN_004017f0(puVar14);
          local_5a50 = CONCAT31(local_5a50._1_3_,0x13);
          FUN_00401210();
          goto LAB_0058c079;
        }
      }
      pwVar10[0xfd] = L'\0';
    }
LAB_0058c079:
    puVar3 = *(undefined4 **)(*(int *)(in_ECX + 0x3208) + 0x18);
    if ((puVar3 == (undefined4 *)0x0) || (*(int *)(*(int *)(in_ECX + 200) + 0x3c) != 0)) {
      iVar5 = *(int *)(in_ECX + 200);
      if (*(int *)(iVar5 + 0x3c) != 0) {
        if (*(int *)(iVar5 + 0x34) == 0) {
          pwVar10 = (wchar_t *)(iVar5 + 0x38);
        }
        else {
          pwVar10 = *(wchar_t **)(iVar5 + 0x34);
        }
        eVar12 = __wsplitpath_s(pwVar10,(wchar_t *)local_10,3,local_810,0x100,&local_210,0x100,
                                local_610,0x100);
        if (eVar12 == 0) {
          local_5a34 = (wchar_t *)0x0;
          local_5a2c = 0;
          local_5a30[0] = L'\0';
          local_5a50._0_1_ = 0x16;
          pwVar10 = local_5a1c;
          if (local_5a1c == (wchar_t *)0x0) {
            pwVar10 = local_5a18;
          }
          FUN_004111b0("%s%s%s",local_10,local_810,pwVar10,0);
          FUN_00402180(&local_210);
          local_5a50._0_1_ = 0x17;
          FUN_00402180(&local_210);
          local_5a50._0_1_ = 0x18;
          FUN_00405390(local_610);
          pwVar10 = local_5a1c;
          if (local_5a1c == (wchar_t *)0x0) {
            pwVar10 = local_5a18;
          }
          pwVar11 = local_5a28;
          if (local_5a28 == (wchar_t *)0x0) {
            pwVar11 = local_5a24;
          }
          iVar5 = __wcsicmp(pwVar11,pwVar10);
          if (iVar5 != 0) {
            pwVar10 = local_5a1c;
            if (local_5a1c == (wchar_t *)0x0) {
              pwVar10 = local_5a18;
            }
            pwVar10 = _wcschr(pwVar10,L'+');
            if (pwVar10 == (wchar_t *)0x0) {
LAB_0058c284:
              pwVar10 = local_5a28;
              if (local_5a28 == (wchar_t *)0x0) {
                pwVar10 = local_5a24;
              }
              pwVar10 = _wcsrchr(pwVar10,L'_');
              if (pwVar10 != (wchar_t *)0x0) {
                FUN_00420390(&local_5a28);
                local_5a50._0_1_ = 0x1a;
                pwVar10 = local_5a40;
                if (local_5a40 == (wchar_t *)0x0) {
                  pwVar10 = local_5a3c;
                }
                pwVar10 = _wcsrchr(pwVar10,L'_');
                if (pwVar10 != (wchar_t *)0x0) {
                  *pwVar10 = L'\0';
                  do {
                    pwVar10 = pwVar10 + 1;
                    wVar2 = *pwVar10;
                    if ((wVar2 == L'\0') || (wVar2 == L'.')) {
                      if (*pwVar10 == L'.') {
                        FUN_00405390(pwVar10);
                        pwVar10 = local_5a1c;
                        if (local_5a1c == (wchar_t *)0x0) {
                          pwVar10 = local_5a18;
                        }
                        pwVar11 = local_5a40;
                        if (local_5a40 == (wchar_t *)0x0) {
                          pwVar11 = local_5a3c;
                        }
                        iVar5 = __wcsicmp(pwVar11,pwVar10);
                        if (iVar5 == 0) goto LAB_0058c22b;
                      }
                      break;
                    }
                    iVar5 = _isdigit((uint)(ushort)wVar2);
                  } while (iVar5 != 0);
                }
                local_5a50._0_1_ = 0x18;
                FUN_00401210();
              }
              if ((*(uint *)(*(int *)(in_ECX + 200) + 0x164) & 0x2000) != 0) {
                pwVar10 = local_5a34;
                if (local_5a34 == (wchar_t *)0x0) {
                  pwVar10 = local_5a30;
                }
                eVar12 = __wsplitpath_s(pwVar10,(wchar_t *)local_10,3,local_810,0x100,&local_210,
                                        0x100,local_410,0x100);
                if ((eVar12 == 0) && (iVar5 = __wcsicmp(local_610,local_410), iVar5 != 0))
                goto LAB_0058c237;
              }
              if (*(int *)(*(int *)(in_ECX + 0x3208) + 0x148) == 0) {
                pwVar10 = local_5a28;
                if (local_5a28 == (wchar_t *)0x0) {
                  pwVar10 = local_5a24;
                }
                pwVar11 = L"manuallogin.html";
                do {
                  wVar2 = *pwVar10;
                  bVar18 = (ushort)wVar2 < (ushort)*pwVar11;
                  if (wVar2 != *pwVar11) {
LAB_0058c47a:
                    iVar5 = (1 - (uint)bVar18) - (uint)(bVar18 != 0);
                    goto LAB_0058c47f;
                  }
                  if (wVar2 == L'\0') break;
                  wVar2 = pwVar10[1];
                  bVar18 = (ushort)wVar2 < (ushort)pwVar11[1];
                  if (wVar2 != pwVar11[1]) goto LAB_0058c47a;
                  pwVar10 = pwVar10 + 2;
                  pwVar11 = pwVar11 + 2;
                } while (wVar2 != L'\0');
                iVar5 = 0;
LAB_0058c47f:
                if (iVar5 == 0) {
                  pwVar10 = local_5a34;
                  if (local_5a34 == (wchar_t *)0x0) {
                    pwVar10 = local_5a30;
                  }
                  eVar12 = __wsplitpath_s(pwVar10,(wchar_t *)local_10,3,local_810,0x100,&local_210,
                                          0x100,local_410,0x100);
                  if ((eVar12 == 0) &&
                     ((iVar5 = __wcsicmp(local_410,L".htm"), iVar5 == 0 ||
                      (iVar5 = __wcsicmp(local_410,L".html"), iVar5 == 0)))) goto LAB_0058c237;
                }
                pwVar10 = local_5a34;
                if (local_5a34 == (wchar_t *)0x0) {
                  pwVar10 = local_5a30;
                }
                eVar12 = __wsplitpath_s(pwVar10,(wchar_t *)local_10,3,local_810,0x100,&local_210,
                                        0x100,local_a10,0x100);
                if ((eVar12 == 0) && (iVar5 = __wcsicmp(local_610,local_a10), iVar5 == 0)) {
                  if ((*(char *)(*(int *)(in_ECX + 200) + 0x19b) != '\0') ||
                     ((*(byte *)(*(int *)(in_ECX + 200) + 0x164) & 0x40) != 0)) goto LAB_0058c237;
                  pwVar10 = &local_210;
                  do {
                    wVar2 = *pwVar10;
                    pwVar10 = pwVar10 + 1;
                  } while (wVar2 != L'\0');
                  pwVar11 = local_5a28;
                  if (local_5a28 == (wchar_t *)0x0) {
                    pwVar11 = local_5a24;
                  }
                  iVar5 = _wcsncmp(&local_210,pwVar11,(int)pwVar10 - (int)local_20e >> 1);
                  if (iVar5 == 0) {
                    pwVar10 = &local_210;
                    do {
                      wVar2 = *pwVar10;
                      pwVar10 = pwVar10 + 1;
                    } while (wVar2 != L'\0');
                    pwVar11 = local_5a28;
                    if (local_5a28 == (wchar_t *)0x0) {
                      pwVar11 = local_5a24;
                    }
                    pwVar15 = pwVar11 + 1;
                    do {
                      wVar2 = *pwVar11;
                      pwVar11 = pwVar11 + 1;
                    } while (wVar2 != L'\0');
                    if ((uint)((int)pwVar10 - (int)local_20e >> 1) <
                        (uint)((int)pwVar11 - (int)pwVar15 >> 1)) {
                      pwVar10 = &local_210;
                      do {
                        wVar2 = *pwVar10;
                        pwVar10 = pwVar10 + 1;
                      } while (wVar2 != L'\0');
                      if (local_5a28 == (wchar_t *)0x0) {
                        local_5a28 = local_5a24;
                      }
                      if (local_5a28[(int)pwVar10 - (int)local_20e >> 1] == L'_') goto LAB_0058c237;
                    }
                  }
                }
                if (*(char *)(*(int *)(in_ECX + 200) + 0x352) == '\0') {
                  if (1 < *(int *)(in_ECX + 0x32ac)) goto LAB_0058c237;
                  local_5a64 = (char *)0x4;
                  LVar13 = RegQueryValueExA(DAT_00765c48,"notaskfn2",(LPDWORD)0x0,(LPDWORD)0x0,
                                            (LPBYTE)&local_5a68,(LPDWORD)&local_5a64);
                  if ((LVar13 != 0) || (local_5a68 != (char *)0x1)) {
                    local_5a40 = (wchar_t *)0x0;
                    local_5a38 = 0;
                    local_5a3c[0] = L'\0';
                    local_5a50._0_1_ = 0x1b;
                    if (local_5a1c == (wchar_t *)0x0) {
                      local_5a1c = local_5a18;
                    }
                    puVar14 = local_5a4c;
                    if (local_5a4c == (undefined2 *)0x0) {
                      puVar14 = local_5a48;
                    }
                    FUN_004111b0(DAT_00766d3c,puVar14,local_610,local_5a1c,DAT_00765d80);
                    pwVar10 = local_5a40;
                    if (local_5a40 == (wchar_t *)0x0) {
                      pwVar10 = local_5a3c;
                    }
                    iVar5 = FUN_00474a80(0,pwVar10,"Internet Download Manager",0x50024);
                    if (iVar5 == 7) goto LAB_0058c22b;
                    local_5a50._0_1_ = 0x18;
                    FUN_00401210();
                  }
                }
              }
              else {
                pwVar10 = local_5a34;
                if (local_5a34 == (wchar_t *)0x0) {
                  pwVar10 = local_5a30;
                }
                eVar12 = __wsplitpath_s(pwVar10,(wchar_t *)local_10,3,local_810,0x100,&local_210,
                                        0x100,local_410,0x100);
                if ((eVar12 == 0) && (iVar5 = __wcsicmp(local_610,local_410), iVar5 == 0))
                goto LAB_0058c237;
              }
              pwVar10 = local_5a34;
              if (local_5a34 == (wchar_t *)0x0) {
                pwVar10 = local_5a30;
              }
              FUN_005d2b00(pwVar10);
              iVar5 = *(int *)(*(int *)(in_ECX + 0x3208) + 0xb8);
              if (iVar5 != 0) {
                FUN_005d4830(iVar5);
              }
              puVar3 = *(undefined4 **)(*(int *)(in_ECX + 0x3208) + 0x14);
              if (puVar3 != (undefined4 *)0x0) {
                SendMessageA((HWND)*puVar3,0x111,0x13c9,
                             *(LPARAM *)(*(int *)(in_ECX + 0x3208) + 0xb8));
              }
            }
            else {
              pwVar10 = local_5a1c;
              if (local_5a1c == (wchar_t *)0x0) {
                pwVar10 = local_5a18;
              }
              FUN_00402180(pwVar10);
              local_5a50._0_1_ = 0x19;
              FUN_0040f4e0(0x2b,0x20);
              pwVar10 = local_5a40;
              if (local_5a40 == (wchar_t *)0x0) {
                pwVar10 = local_5a3c;
              }
              pwVar11 = local_5a28;
              if (local_5a28 == (wchar_t *)0x0) {
                pwVar11 = local_5a24;
              }
              iVar5 = __wcsicmp(pwVar11,pwVar10);
              if (iVar5 != 0) {
                local_5a50._0_1_ = 0x18;
                FUN_00401210();
                goto LAB_0058c284;
              }
LAB_0058c22b:
              local_5a50._0_1_ = 0x18;
              FUN_00401210();
            }
          }
LAB_0058c237:
          local_5a50._0_1_ = 0x17;
          FUN_00401210();
          local_5a50._0_1_ = 0x16;
          FUN_00401210();
          local_5a50 = CONCAT31(local_5a50._1_3_,0x13);
          FUN_00401210();
        }
      }
    }
    else if ((*(uint *)(*(int *)(in_ECX + 200) + 0x164) & 0x2000) == 0) {
      if (local_5a1c == (wchar_t *)0x0) {
        local_5a1c = local_5a18;
      }
      SendMessageA((HWND)*puVar3,0x111,0x13bc,(LPARAM)local_5a1c);
    }
    else {
      if (local_5a1c == (wchar_t *)0x0) {
        local_5a1c = local_5a18;
      }
      SendMessageA((HWND)*puVar3,0x111,0x148f,(LPARAM)local_5a1c);
    }
  }
  local_5a50._0_1_ = 9;
  FUN_00401210();
  local_5a50 = (uint)local_5a50._1_3_ << 8;
  FUN_00402570();
LAB_0058a9e8:
  ExceptionList = local_5a58;
  __security_check_cookie(local_8 ^ (uint)&local_5a4c);
  return;
LAB_0058ad80:
  if (*pcVar17 == '\0') goto LAB_0058ae4c;
  if (*pcVar17 == '=') {
    if ((*pcVar17 == '\0') || (pcVar17 = pcVar17 + 1, *pcVar17 == '\0')) goto LAB_0058ae4c;
    iVar5 = FUN_0057ecc0(pcVar17);
    pcVar17 = pcVar17 + iVar5;
    if (*pcVar17 == '\0') goto LAB_0058ae4c;
    if (*pcVar17 == '\"') {
      pcVar17 = pcVar17 + 1;
      if (*pcVar17 == '\0') goto LAB_0058ae4c;
      pcVar6 = _strchr(pcVar17,0x22);
LAB_0058adf0:
      if (pcVar6 == (char *)0x0) goto LAB_0058ae4c;
    }
    else {
      pcVar6 = _strstr(pcVar17,"\r\n");
      if (pcVar6 == (char *)0x0) {
        pcVar6 = _strstr(pcVar17,"\n");
        goto LAB_0058adf0;
      }
    }
    local_5a68 = pcVar6 + -(int)pcVar17;
    if (0 < (int)local_5a68) {
      pvVar7 = (void *)FUN_00402850(local_5a68);
      _memcpy(pvVar7,pcVar17,(size_t)local_5a68);
    }
    goto LAB_0058ae4c;
  }
  pcVar17 = pcVar17 + 1;
  goto LAB_0058ad80;
}


