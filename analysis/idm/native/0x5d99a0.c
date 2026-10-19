/* REA ev_73fc19b173c0231cbf94b2358541fe8865f5abe6ccbd11126f61dfb4a4182987; decompiler output, not original source. */

void FUN_005d99a0(byte *param_1)

{
  byte bVar1;
  int iVar2;
  byte *pbVar3;
  int iVar4;
  int in_ECX;
  byte *pbVar5;
  int iVar6;
  bool bVar7;
  int local_14;
  int local_10;
  void *local_c;
  undefined1 *puStack_8;
  undefined4 local_4;
  
  pbVar3 = param_1;
  local_4 = 0xffffffff;
  puStack_8 = &LAB_0066c2b0;
  local_c = ExceptionList;
  ExceptionList = &local_c;
  for (iVar6 = 0; ((bVar1 = param_1[iVar6], bVar1 == 0x20 || (bVar1 == 9)) || (bVar1 == 0xd));
      iVar6 = iVar6 + 1) {
  }
  pbVar5 = param_1 + iVar6;
  iVar2 = __memicmp(pbVar5,"Cookie2:",8);
  if (iVar2 == 0) {
    FUN_005d8470(pbVar5 + 8,in_ECX + 900);
  }
  else {
    iVar2 = __memicmp(pbVar5,"Cookie:",7);
    if (iVar2 == 0) {
      FUN_005d8470(pbVar5 + 7,in_ECX + 0x38c);
    }
    else {
      iVar2 = __memicmp(pbVar5,"Accept-Language:",0x10);
      if (iVar2 == 0) {
        FUN_005d8470(pbVar5 + 0x10,in_ECX + 0x388);
      }
      else {
        iVar2 = __memicmp(pbVar5,"Accept-Charset:",0xf);
        if (iVar2 == 0) {
          FUN_005d8470(pbVar5 + 0xf,in_ECX + 0x398);
        }
        else {
          iVar2 = __memicmp(pbVar5,"Origin:",7);
          if (iVar2 == 0) {
            FUN_005d8470(pbVar5 + 7,in_ECX + 0x390);
          }
          else {
            iVar2 = __memicmp(pbVar5,"Accept:",7);
            if (iVar2 == 0) {
              FUN_005d8470(pbVar5 + 7,in_ECX + 0x394);
            }
            else {
              iVar2 = __memicmp(pbVar5,"Accept-Encoding:",0x10);
              if (iVar2 == 0) {
                FUN_005d8470(pbVar5 + 0x10,in_ECX + 0x418);
              }
              else {
                iVar2 = __memicmp(pbVar5,"Content-Type:",0xd);
                if (iVar2 == 0) {
                  if (*(char *)(in_ECX + 0x19d) != '\0') {
                    FUN_005d8470(pbVar5 + 0xd,in_ECX + 0xd0);
                  }
                }
                else {
                  iVar2 = __memicmp(pbVar5,"Range:",6);
                  if (iVar2 == 0) {
                    FUN_00402500();
                    local_4 = 0;
                    FUN_005d8470(pbVar5 + 6,&param_1);
                    pbVar5 = &DAT_006bc41c;
                    pbVar3 = param_1;
                    do {
                      bVar1 = *pbVar3;
                      bVar7 = bVar1 < *pbVar5;
                      if (bVar1 != *pbVar5) {
LAB_005d9bb0:
                        iVar6 = (1 - (uint)bVar7) - (uint)(bVar7 != 0);
                        goto LAB_005d9bb5;
                      }
                      if (bVar1 == 0) break;
                      bVar1 = pbVar3[1];
                      bVar7 = bVar1 < pbVar5[1];
                      if (bVar1 != pbVar5[1]) goto LAB_005d9bb0;
                      pbVar3 = pbVar3 + 2;
                      pbVar5 = pbVar5 + 2;
                    } while (bVar1 != 0);
                    iVar6 = 0;
LAB_005d9bb5:
                    if (iVar6 == 0) {
                      *(undefined1 *)(in_ECX + 0x1b9) = 1;
                    }
                    else {
                      iVar6 = _strncmp((char *)param_1,"bytes=0-",8);
                      if (iVar6 == 0) {
                        local_14 = 0;
                        local_10 = 0;
                        iVar6 = FID_conflict__sscanf((char *)(param_1 + 8),"%I64d",&local_14);
                        if ((iVar6 != 0) && (local_14 != 0 || local_10 != 0)) {
                          *(int *)(in_ECX + 0x310) = local_14;
                          *(int *)(in_ECX + 0x314) = local_10;
                        }
                      }
                    }
                    *(undefined1 *)(in_ECX + 0x1ba) = 1;
                  }
                  else {
                    iVar2 = __memicmp(pbVar5,"Proxy-Authorization:",0x14);
                    if (iVar2 != 0) {
                      iVar2 = __memicmp(pbVar5,"Authorization:",0xe);
                      if (iVar2 == 0) {
                        iVar2 = FUN_005d0780(pbVar5 + 0xe);
                        iVar4 = __memicmp(pbVar3 + iVar2 + iVar6 + 0xe,"Bearer ",7);
                        if ((iVar4 != 0) &&
                           (iVar6 = __memicmp(pbVar3 + iVar2 + iVar6 + 0xe,"token ",6), iVar6 != 0))
                        {
                          ExceptionList = local_c;
                          return;
                        }
                        FUN_00402f70(in_ECX + 0x3a0,&DAT_0067def0,pbVar3);
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Content-Length:",0xf);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Accept-Encoding:",0x10);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"User-Agent:",0xb);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Host:",5);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Connection:",0xb);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Range:",6);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Referer:",8);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Cache-Control:",0xe);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Pragma:",7);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"If-Modified-Since:",0x12);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"If-Unmodified-Since:",0x14);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"If-Range:",9);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"If-Match:",9);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"If-None-Match:",0xe);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Upgrade:",8);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,&DAT_006bc334,3);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Trailer:",8);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,&DAT_006bc320,4);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Upgrade-Insecure-Requests:",0x1a);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      iVar6 = __memicmp(pbVar5,"Cookie:",7);
                      if (iVar6 == 0) {
                        ExceptionList = local_c;
                        return;
                      }
                      FUN_004030b0(pbVar3);
                      ExceptionList = local_c;
                      return;
                    }
                    FUN_00402500();
                    local_4 = 1;
                    FUN_005d8470(pbVar5 + 0x14,&param_1);
                    iVar6 = __memicmp(param_1,"Basic ",6);
                    if (iVar6 == 0) {
                      FUN_00402f70(in_ECX + 0x3b0,&DAT_0067def0,param_1 + 6);
                    }
                  }
                  local_4 = 0xffffffff;
                  FUN_00402570();
                }
              }
            }
          }
        }
      }
    }
  }
  ExceptionList = local_c;
  return;
}


