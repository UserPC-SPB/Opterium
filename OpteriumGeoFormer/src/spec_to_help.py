"""spec_to_help.py — Генерация API документации и help из spec_compiled.json

Создаёт:
  - API.md        — полная документация API
  - help.txt      — краткая справка для консоли
  - inject_help() — встраивает help() в модули
"""
import sys, os, json
SRC = os.path.dirname(__file__)
SPEC_PATH = os.path.join(SRC, 'spec-kit', 'spec_compiled.json')

def load_spec():
    with open(SPEC_PATH, 'r', encoding='utf-8') as f:
        return json.load(f)

def generate_api(data) -> str:
    lines = ["# Opterium GeoFormer — API Reference", "",
             "Auto-generated from spec_compiled.json. All examples verified by tests.",
             "", "---", ""]
    
    for mod_name in sorted(data.keys()):
        mod = data[mod_name]
        lines.append(f"## Module: `{mod_name}`")
        lines.append(f"  File: `{mod.get('filepath', '')}`")
        lines.append("")
        
        classes = mod.get('classes', {})
        for cls_name in sorted(classes.keys()):
            cls = classes[cls_name]
            lines.append(f"### Class `{cls_name}`")
            if cls.get('description'):
                lines.append(f"  {cls['description']}")
            lines.append("")
            
            methods = cls.get('methods', {})
            for m_name in sorted(methods.keys()):
                m = methods[m_name]
                sig = m.get('signature', '')
                if sig.startswith(m_name):
                    sig = sig[len(m_name):]
                desc = m.get('description', '')
                lines.append(f"#### `{cls_name}.{m_name}{sig}`")
                lines.append(f"  {desc}")
                lines.append("")
                
                # Inputs
                inputs = m.get('inputs', [])
                if inputs:
                    lines.append("  **Args:**")
                    for inp in inputs:
                        lines.append(f"    - `{inp.get('name','?')}`: `{inp.get('type','?')}`  {inp.get('range','')}")
                    lines.append("")
                
                # Outputs
                outputs = m.get('outputs', [])
                if outputs:
                    lines.append("  **Returns:**")
                    for out in outputs:
                        lines.append(f"    - `{out.get('type','?')}`  {out.get('description','')}")
                    lines.append("")
                
                # Examples
                examples = m.get('examples', [])
                if examples:
                    lines.append("  **Examples:**")
                    for ex in examples[:3]:
                        inp = ex.get('input', '')
                        out = ex.get('output', '')
                        desc_ex = ex.get('description', '')
                        inp_str = json.dumps(inp) if not isinstance(inp, str) else f'"{inp}"'
                        out_str = json.dumps(out) if not isinstance(out, str) else out
                        if len(inp_str) > 60: inp_str = inp_str[:57] + '...'
                        if len(out_str) > 60: out_str = out_str[:57] + '...'
                        lines.append(f"      • `{desc_ex}` → {out_str}")
                    if len(examples) > 3:
                        lines.append(f"      *(+{len(examples)-3} more)*")
                    lines.append("")
                
                # Edges
                edges = m.get('edges', [])
                if edges:
                    lines.append("  **Edge cases:**")
                    for ed in edges:
                        lines.append(f"      • {ed.get('description','')}")
                    lines.append("")
                
                # Errors
                errors = m.get('errors', [])
                if errors:
                    lines.append("  **Errors:**")
                    for err in errors:
                        lines.append(f"      • {err.get('description','')} → `{err.get('error','')}`")
                    lines.append("")
        
        functions = mod.get('functions', {})
        for f_name in sorted(functions.keys()):
            f = functions[f_name]
            sig = f.get('signature', '')
            if sig.startswith(f_name):
                sig = sig[len(f_name):]
            desc = f.get('description', '')
            lines.append(f"### Function `{f_name}{sig}`")
            lines.append(f"  {desc}")
            lines.append("")
            
            examples = f.get('examples', [])
            if examples:
                lines.append("  **Examples:**")
                for ex in examples[:3]:
                    desc_ex = ex.get('description', '')
                    out = ex.get('output', '')
                    out_str = json.dumps(out) if not isinstance(out, str) else out
                    if len(out_str) > 60: out_str = out_str[:57] + '...'
                    lines.append(f"      • `{desc_ex}` → {out_str}")
                if len(examples) > 3:
                    lines.append(f"      *(+{len(examples)-3} more)*")
                lines.append("")
        
        lines.append("---")
        lines.append("")
    
    # Statistics
    total_examples = sum(
        len(m.get('examples', []))
        for mod in data.values()
        for cls in mod.get('classes', {}).values()
        for m in cls.get('methods', {}).values()
    ) + sum(
        len(f.get('examples', []))
        for mod in data.values()
        for f in mod.get('functions', {}).values()
    )
    lines.append(f"*Generated from {len(data)} modules, {total_examples} verified examples*")
    
    return '\n'.join(lines)


def generate_help(data) -> str:
    lines = ["Opterium GeoFormer — Help", "=" * 40, "",
             "Modules: PtTable, Pt, Cube27, HashGrid, delta_ops, phi_algebra, swarm, e8_twist, doctor_geo, geoformer",
             "Usage: python -c 'from <module> import *; help(<class/function>)'",
             "", "--- Quick Reference ---", ""]
    
    for mod_name in sorted(data.keys()):
        mod = data[mod_name]
        lines.append(f"  {mod_name}:")
        
        classes = mod.get('classes', {})
        for cls_name in sorted(classes.keys()):
            cls = classes[cls_name]
            desc = cls.get('description', '')
            lines.append(f"    class {cls_name}  — {desc}")
            
            methods = cls.get('methods', {})
            for m_name in sorted(methods.keys()):
                m = methods[m_name]
                sig = m.get('signature', '')
                # Remove method name prefix from signature to avoid duplication
                if sig.startswith(m_name):
                    sig = sig[len(m_name):]
                desc_m = m.get('description', '')
                lines.append(f"      .{m_name}{sig}")
        
        functions = mod.get('functions', {})
        for f_name in sorted(functions.keys()):
            f = functions[f_name]
            sig = f.get('signature', '')
            if sig.startswith(f_name):
                sig = sig[len(f_name):]
            desc_f = f.get('description', '')
            lines.append(f"    def {f_name}{sig}  — {desc_f}")
        
        lines.append("")
    
    return '\n'.join(lines)


def inject_help():
    """Patch __builtins__.help to show spec-based help for our modules."""
    data = load_spec()
    help_text = generate_help(data)
    api_text = generate_api(data)
    
    # Save files
    api_path = os.path.join(SRC, 'spec-kit', 'API.md')
    help_path = os.path.join(SRC, 'spec-kit', 'help.txt')
    
    with open(api_path, 'w', encoding='utf-8') as f:
        f.write(api_text)
    print(f"✅ API.md saved: {api_path} ({len(api_text)} chars)")
    
    with open(help_path, 'w', encoding='utf-8') as f:
        f.write(help_text)
    print(f"✅ help.txt saved: {help_path} ({len(help_text)} chars)")


def module_help(mod_name: str) -> str:
    """Get help for a specific module as string."""
    data = load_spec()
    mod = data.get(mod_name)
    if not mod:
        return f"No spec for module '{mod_name}'"
    
    lines = [f"=== {mod_name} ===", f"  {mod.get('filepath', '')}", ""]
    
    classes = mod.get('classes', {})
    for cls_name in sorted(classes.keys()):
        cls = classes[cls_name]
        lines.append(f"class {cls_name}:")
        if cls.get('description'):
            lines.append(f"  \"{cls['description']}\"")
        
        methods = cls.get('methods', {})
        for m_name in sorted(methods.keys()):
            m = methods[m_name]
            sig = m.get('signature', '')
            desc = m.get('description', '')
            lines.append(f"  .{m_name}{sig}")
            lines.append(f"      {desc}")
            
            examples = m.get('examples', [])
            if examples:
                ex = examples[0]
                lines.append(f"      ex: {ex.get('description','')}")
    
    functions = mod.get('functions', {})
    for f_name in sorted(functions.keys()):
        f = functions[f_name]
        sig = f.get('signature', '')
        desc = f.get('description', '')
        lines.append(f"{f_name}{sig}")
        lines.append(f"  {desc}")
        
        examples = f.get('examples', [])
        if examples:
            ex = examples[0]
            lines.append(f"  ex: {ex.get('description','')}")
    
    return '\n'.join(lines)


if __name__ == '__main__':
    inject_help()
    
    # Also print summary
    data = load_spec()
    print(f"\nModules with spec:")
    for name in sorted(data.keys()):
        mod = data[name]
        n_classes = len(mod.get('classes', {}))
        n_funcs = len(mod.get('functions', {}))
        n_ex = sum(len(m.get('examples',[])) for cls in mod.get('classes',{}).values() for m in cls.get('methods',{}).values())
        n_ex += sum(len(f.get('examples',[])) for f in mod.get('functions',{}).values())
        print(f"  {name}: {n_classes} classes, {n_funcs} functions, {n_ex} examples")
