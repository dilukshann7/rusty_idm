"""Read-only PE imports and bounded string candidates; no execution or disassembly."""
import hashlib
import json
import re
import struct
from pathlib import Path

INSTALL = Path(r"C:\Program Files (x86)\Internet Download Manager")
OUT = Path(__file__).parent


def inspect(name):
    path = INSTALL / name
    data = path.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    count, optional_size = struct.unpack_from("<H", data, pe + 6)[0], struct.unpack_from("<H", data, pe + 20)[0]
    opt = pe + 24
    magic = struct.unpack_from("<H", data, opt)[0]
    assert magic == 0x10B, "this helper intentionally supports only the inspected PE32 targets"
    base = struct.unpack_from("<I", data, opt + 28)[0]
    sections = []
    for n in range(count):
        offset = opt + optional_size + n * 40
        label = data[offset:offset + 8].rstrip(b"\0").decode("ascii", "replace")
        virtual_size, rva, size, raw = struct.unpack_from("<IIII", data, offset + 8)
        sections.append(dict(name=label, rva=rva, virtual_size=virtual_size, raw=raw, size=size))

    def position(rva):
        for section in sections:
            if section["rva"] <= rva < section["rva"] + section["size"]:
                return section["raw"] + rva - section["rva"]
        raise ValueError(f"unmapped RVA {rva:x}")

    def cstring(offset):
        return data[offset:data.index(b"\0", offset)].decode("ascii", "replace")

    import_rva, import_size = struct.unpack_from("<II", data, opt + 104)
    imports = []
    if import_rva:
        offset = position(import_rva)
        while any(data[offset:offset + 20]):
            original, _, _, module, first = struct.unpack_from("<IIIII", data, offset)
            dll = cstring(position(module))
            thunk = position(original or first)
            n = 0
            while (value := struct.unpack_from("<I", data, thunk + n * 4)[0]):
                symbol = f"ordinal:{value & 0xFFFF}" if value & 0x80000000 else cstring(position(value) + 2)
                imports.append(dict(module=dll, symbol=symbol, iat_va=hex(base + first + n * 4)))
                n += 1
            offset += 20
    candidates = []
    pattern = re.compile(r"(?i)(range|etag|modified|resume|retry|speed|queue|\.sav|\.part|\.tmp|appdata|software\\|idm|pipe|WM_COPYDATA)")
    for encoding, expression in [("ascii", rb"[\x20-\x7e]{5,500}"), ("utf-16le", rb"(?:[\x20-\x7e]\x00){5,500}")]:
        for match in re.finditer(expression, data):
            text = match[0].decode(encoding)
            if pattern.search(text):
                va = None
                section_name = None
                for section in sections:
                    if section["raw"] <= match.start() < section["raw"] + section["size"]:
                        va = hex(base + section["rva"] + match.start() - section["raw"])
                        section_name = section["name"]
                        break
                candidates.append(dict(offset=match.start(), va=va, section=section_name, encoding=encoding, value=text))
    result = dict(source="local-static-PE-parser-not-REA", path=str(path), sha256=hashlib.sha256(data).hexdigest(),
                  image_base=hex(base), sections=sections, imports=imports, candidates=candidates,
                  limitations=["Static import table only; dynamic imports unresolved", "String candidates do not establish execution or semantics", "ASCII and ASCII-subset UTF-16LE scans are bounded to 5..500-character fragments"])
    (OUT / (name + ".pe.json")).write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(name, "imports", len(imports), "candidates", len(candidates))
    print(json.dumps([item for item in candidates if item["section"] != ".rsrc" and re.search(r"(?i)(range|etag|modified|pipe|software\\|\.sav|\.part)", item["value"])][:35], indent=2))


if __name__ == "__main__":
    inspect("IDMan.exe")
    inspect("IDMMsgHost.exe")
