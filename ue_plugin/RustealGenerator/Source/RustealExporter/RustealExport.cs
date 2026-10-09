#nullable disable

using EpicGames.Core;
using EpicGames.UHT.Tables;
using EpicGames.UHT.Types;
using EpicGames.UHT.Utils;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using JsonObject = System.Text.Json.Nodes.JsonObject;
using JsonArray = System.Text.Json.Nodes.JsonArray;

namespace Rusteal;

[UnrealHeaderTool]
public static class RustealExport
{
    #region Filter Constants

    private const EPropertyFlags NoExportPropFlags =
        EPropertyFlags.NativeAccessSpecifierPrivate |
        EPropertyFlags.NativeAccessSpecifierProtected;

    private const EFunctionFlags NoExportFuncFlags =
        EFunctionFlags.NetServer |
        EFunctionFlags.NetRequest |
        EFunctionFlags.NetResponse |
        EFunctionFlags.Private |
        EFunctionFlags.Delegate;

    private const EPropertyFlags ScriptExposedPropFlags =
        EPropertyFlags.BlueprintVisible | EPropertyFlags.BlueprintAssignable;

    private const EFunctionFlags ScriptExposedFuncFlags =
        EFunctionFlags.BlueprintCallable | EFunctionFlags.BlueprintEvent;

    private static readonly HashSet<string> NeedRegisterStruct =
    [
        "Rotator", "Quat", "Transform", "Color", "LinearColor", "Plane",
        "Vector", "Vector2D", "Vector4", "RandomStream", "Guid", "Box2D",
        "FallbackStruct", "FloatRangeBound", "FloatRange", "Int32RangeBound",
        "Int32Range", "FloatInterval", "Int32Interval", "FrameNumber",
        "SoftObjectPath", "SoftClassPath", "PrimaryAssetType", "PrimaryAssetId",
        "DateTime", "PolyglotTextData",
    ];

    #endregion

    #region Entry Point

    [UhtExporter(
        Name = "Rusteal",
        Description = "Export UE reflection data to JSON for Rusteal Rust codegen",
        Options = UhtExporterOptions.Default,
        ModuleName = "RustealGenerator")]
    public static void Generate(IUhtExportFactory factory)
    {
        new Exporter(factory).Run();
    }

    #endregion

    #region Exporter

    private sealed class Exporter(IUhtExportFactory factory)
    {
        private readonly IUhtExportFactory _factory = factory;
        private readonly UhtSession _session = factory.Session;
        private readonly List<JsonObject> _classes = [];
        private readonly List<JsonObject> _structs = [];
        private readonly List<JsonObject> _enums = [];
        private readonly HashSet<string> _exportedClassNames = [];
        private readonly HashSet<string> _exportedStructNames = [];
        private readonly HashSet<string> _exportedEnumNames = [];

        public void Run()
        {
            CollectTypes();
            WriteJsonFiles();
        }

        private void CollectTypes()
        {
            foreach (UhtModule module in _session.Modules)
            {
                string package = module.ShortName;

                foreach (UhtHeaderFile header in module.Headers)
                {
                    foreach (UhtType type in header.Children)
                    {
                        switch (type)
                        {
                            case UhtClass classObj
                                when classObj.ClassType is UhtClassType.Class or UhtClassType.Interface:
                                TryExportClass(classObj, package, header);
                                break;

                            case UhtScriptStruct structObj:
                                TryExportStruct(structObj, package, header);
                                break;

                            case UhtEnum enumObj:
                                TryExportEnum(enumObj, package);
                                break;
                        }
                    }
                }
            }
        }

        private void TryExportClass(UhtClass classObj, string package, UhtHeaderFile header)
        {
            string name = StripPrefix(classObj.SourceName);

            if (!_exportedClassNames.Add(name))
                return;

            if (!ShouldExportClass(classObj))
            {
                _exportedClassNames.Remove(name);
                return;
            }

            var props = new JsonArray();

            foreach (UhtProperty prop in classObj.Properties)
            {
                if (ShouldExportProperty(prop))
                    props.Add(ExportProperty(prop));
            }

            var funcs = new JsonArray();

            foreach (UhtFunction func in classObj.Functions)
            {
                if (ShouldExportFunction(func))
                    funcs.Add(ExportFunction(func));
            }

            var interfaces = new JsonArray();

            foreach (UhtStruct baseStruct in classObj.Bases)
            {
                if (baseStruct is UhtClass iface)
                    interfaces.Add(StripPrefix(iface.SourceName));
            }

            string superName = classObj.SuperClass != null
                ? StripPrefix(classObj.SuperClass.SourceName)
                : null;

            _classes.Add(new JsonObject
            {
                ["name"] = name,
                ["cpp_name"] = classObj.SourceName,
                ["package"] = package,
                ["header"] = header.IncludeFilePath ?? header.ModuleRelativeFilePath ?? "",
                ["header_public"] = IsPublicHeader(header),
                ["class_flags"] = (long)unchecked((uint)classObj.ClassFlags),
                ["super"] = superName,
                ["interfaces"] = interfaces,
                ["props"] = props,
                ["funcs"] = funcs,
            });
        }

        private static bool IsPublicHeader(UhtHeaderFile header)
        {
            return header.HeaderFileType == UhtHeaderFileType.Classes
                || header.HeaderFileType == UhtHeaderFileType.Public;
        }

        private static bool ShouldExportClass(UhtClass classObj)
        {
            if (classObj.ClassFlags.HasAnyFlags(EClassFlags.Deprecated))
                return false;

            if (!IsApiClass(classObj))
                return false;

            return IsScriptExposed(classObj) || HasScriptExposedFields(classObj);
        }

        private static bool IsApiClass(UhtClass classObj)
        {
            return classObj.ClassFlags.HasAnyFlags(EClassFlags.MinimalAPI)
                || classObj.ClassFlags.HasAnyFlags(EClassFlags.RequiredAPI);
        }

        private void TryExportStruct(UhtScriptStruct structObj, string package, UhtHeaderFile header)
        {
            string name = StripPrefix(structObj.SourceName);

            if (!_exportedStructNames.Add(name))
                return;

            if (!ShouldExportStruct(structObj))
            {
                _exportedStructNames.Remove(name);
                return;
            }

            var props = new JsonArray();

            foreach (UhtProperty prop in structObj.Properties)
            {
                if (ShouldExportProperty(prop))
                    props.Add(ExportProperty(prop));
            }

            string superName = structObj.Super is UhtScriptStruct superStruct
                ? StripPrefix(superStruct.SourceName)
                : null;

            bool hasStaticStruct = !structObj.ScriptStructFlags.HasAnyFlags(EStructFlags.NoExport)
                                || NeedRegisterStruct.Contains(name);

            string structHeader = structObj.ScriptStructFlags.HasAnyFlags(EStructFlags.NoExport)
                ? ""
                : header.IncludeFilePath ?? header.ModuleRelativeFilePath ?? "";

            _structs.Add(new JsonObject
            {
                ["name"] = name,
                ["cpp_name"] = structObj.SourceName,
                ["package"] = package,
                ["header"] = structHeader,
                ["header_public"] = IsPublicHeader(header),
                ["struct_flags"] = (long)unchecked((uint)structObj.ScriptStructFlags),
                ["super"] = superName,
                ["has_static_struct"] = hasStaticStruct,
                ["props"] = props,
            });
        }

        private static bool ShouldExportStruct(UhtScriptStruct structObj)
        {
            return IsScriptExposed(structObj) || HasScriptExposedFields(structObj);
        }

        private void TryExportEnum(UhtEnum enumObj, string package)
        {
            string name = enumObj.SourceName;

            if (!_exportedEnumNames.Add(name))
                return;

            if (!ShouldExportEnum(enumObj))
            {
                _exportedEnumNames.Remove(name);
                return;
            }

            Dictionary<string, long> evaluated = enumObj.EnumValues.Any(v => v.Value == -1)
                ? RustealEnumValues.Evaluate(enumObj)
                : null;

            var pairs = new JsonArray();

            foreach (UhtEnumValue val in enumObj.EnumValues)
            {
                string valName = val.Name;
                int colonIdx = valName.LastIndexOf("::", StringComparison.Ordinal);

                if (colonIdx >= 0)
                    valName = valName[(colonIdx + 2)..];

                long value = val.Value;

                if (value == -1 && evaluated != null && evaluated.TryGetValue(valName, out long computed))
                {
                    value = computed;
                }

                var pair = new JsonArray
                {
                    valName,
                    value
                };

                pairs.Add(pair);
            }

            string underlyingType = enumObj.UnderlyingType switch
            {
                UhtEnumUnderlyingType.Uint8 => "uint8",
                UhtEnumUnderlyingType.Int8 => "int8",
                UhtEnumUnderlyingType.Int16 => "int16",
                UhtEnumUnderlyingType.Uint16 => "uint16",
                UhtEnumUnderlyingType.Int32 => "int32",
                UhtEnumUnderlyingType.Uint32 => "uint32",
                UhtEnumUnderlyingType.Int64 => "int64",
                UhtEnumUnderlyingType.Uint64 => "uint64",
                _ => "uint8",
            };

            int cppForm = enumObj.CppForm switch
            {
                UhtEnumCppForm.Regular => 0,
                UhtEnumCppForm.Namespaced => 1,
                UhtEnumCppForm.EnumClass => 2,
                _ => 0,
            };

            _enums.Add(new JsonObject
            {
                ["name"] = name,
                ["cpp_name"] = name,
                ["package"] = package,
                ["underlying_type"] = underlyingType,
                ["cpp_form"] = cppForm,
                ["pairs"] = pairs,
            });
        }

        private static bool ShouldExportEnum(UhtEnum enumObj)
        {
            return !enumObj.MetaData.ContainsKey("NotBlueprintType");
        }

        private static bool IsScriptExposed(UhtType type)
        {
            UhtType current = type;

            while (current != null)
            {
                if (current.MetaData.ContainsKey("BlueprintType")
                    || current.MetaData.ContainsKey("BlueprintSpawnableComponent"))
                    return true;

                if (current.MetaData.ContainsKey("NotBlueprintType"))
                    return false;

                current = current switch
                {
                    UhtClass c => c.SuperClass,
                    UhtScriptStruct s => s.SuperScriptStruct,
                    _ => null,
                };
            }

            return false;
        }

        private static bool HasScriptExposedFields(UhtStruct structObj)
        {
            UhtStruct current = structObj;

            while (current != null)
            {
                foreach (UhtType child in current.Children)
                {
                    if (child is UhtProperty prop
                        && prop.PropertyFlags.HasAnyFlags(ScriptExposedPropFlags))
                        return true;

                    if (child is UhtFunction func
                        && func.FunctionFlags.HasAnyFlags(ScriptExposedFuncFlags))
                        return true;
                }

                current = current switch
                {
                    UhtClass c => c.SuperClass,
                    UhtScriptStruct s => s.SuperScriptStruct,
                    _ => null,
                };
            }

            return false;
        }

        private static bool ShouldExportProperty(UhtProperty prop)
        {
            if (prop.PropertyFlags.HasAnyFlags(NoExportPropFlags)
                && !prop.PropertyFlags.HasAnyFlags(EPropertyFlags.BlueprintVisible | EPropertyFlags.BlueprintAssignable | EPropertyFlags.Edit))
                return false;

            if (prop.PropertyFlags.HasAnyFlags(EPropertyFlags.Deprecated))
                return false;

            if (prop.PropertyFlags.HasAnyFlags(EPropertyFlags.EditorOnly))
                return false;

            return true;
        }

        private static JsonObject ExportProperty(UhtProperty prop)
        {
            var info = new JsonObject
            {
                ["name"] = prop.SourceName,
                ["type"] = GetPropertyTypeName(prop),
                ["prop_flags"] = unchecked((long)(ulong)prop.PropertyFlags),
                ["array_dim"] = GetArrayDim(prop),
            };

            PopulateSubTypeFields(prop, info);

            info["getter"] = GetMetaOrNull(prop, "BlueprintGetter");
            info["setter"] = GetMetaOrNull(prop, "BlueprintSetter");
            info["default"] = null;

            return info;
        }

        private static JsonObject ExportParam(UhtProperty prop, UhtFunction ownerFunc)
        {
            var info = new JsonObject
            {
                ["name"] = prop.SourceName,
                ["type"] = GetPropertyTypeName(prop),
                ["prop_flags"] = unchecked((long)(ulong)prop.PropertyFlags),
            };

            PopulateSubTypeFields(prop, info);

            string defaultKey = $"CPP_Default_{prop.SourceName}";

            if (ownerFunc.MetaData.TryGetValue(defaultKey, out string defaultVal))
                info["default"] = defaultVal;

            else
                info["default"] = (JsonNode)null;

            return info;
        }

        private static JsonObject ExportInnerProperty(UhtProperty prop)
        {
            var info = new JsonObject
            {
                ["name"] = prop.SourceName,
                ["type"] = GetPropertyTypeName(prop),
                ["prop_flags"] = unchecked((long)(ulong)prop.PropertyFlags),
            };

            PopulateSubTypeFields(prop, info);
            return info;
        }

        private static void PopulateSubTypeFields(UhtProperty prop, JsonObject info)
        {
            info["enum_name"] = (JsonNode)null;
            info["enum_cpp_name"] = (JsonNode)null;
            info["enum_cpp_form"] = (JsonNode)null;
            info["enum_underlying_type"] = (JsonNode)null;
            info["class_name"] = (JsonNode)null;
            info["meta_class_name"] = (JsonNode)null;
            info["struct_name"] = (JsonNode)null;
            info["interface_name"] = (JsonNode)null;
            info["func_info"] = (JsonNode)null;
            info["inner_prop"] = (JsonNode)null;
            info["key_prop"] = (JsonNode)null;
            info["value_prop"] = (JsonNode)null;

            if (prop is UhtByteProperty { Enum: not null } byteProp)
            {
                info["enum_name"] = byteProp.Enum.SourceName;
                info["enum_cpp_name"] = byteProp.Enum.SourceName;
                info["enum_cpp_form"] = (int)byteProp.Enum.CppForm;
            }

            if (prop is UhtEnumProperty enumProp)
            {
                info["enum_name"] = enumProp.Enum.SourceName;
                info["enum_cpp_name"] = enumProp.Enum.SourceName;
                info["enum_cpp_form"] = (int)enumProp.Enum.CppForm;

                info["enum_underlying_type"] = enumProp.UnderlyingProperty != null
                    ? GetUnderlyingTypeName(enumProp.UnderlyingProperty)
                    : "uint8";
            }

            if (prop is UhtObjectPropertyBase objPropBase)
            {
                info["class_name"] = StripPrefix(objPropBase.Class.SourceName);
            }

            if (prop is UhtClassProperty classProp)
            {
                info["meta_class_name"] = StripPrefix(classProp.MetaClass.SourceName);
            }

            if (prop is UhtSoftClassProperty softClassProp)
            {
                info["meta_class_name"] = StripPrefix(softClassProp.MetaClass.SourceName);
            }

            if (prop is UhtInterfaceProperty ifaceProp)
            {
                info["interface_name"] = StripPrefix(ifaceProp.InterfaceClass.SourceName);
            }

            if (prop is UhtStructProperty structProp)
            {
                info["struct_name"] = StripPrefix(structProp.ScriptStruct.SourceName);
            }

            if (prop is UhtDelegateProperty delProp)
            {
                info["func_info"] = ExportDelegateSignature(delProp.Function);
            }

            if (prop is UhtMulticastDelegateProperty mdelProp)
            {
                info["func_info"] = ExportDelegateSignature(mdelProp.Function);
            }

            if (prop is UhtArrayProperty arrProp)
            {
                info["inner_prop"] = ExportInnerProperty(arrProp.ValueProperty);
            }

            if (prop is UhtSetProperty setProp)
            {
                info["element_prop"] = ExportInnerProperty(setProp.ValueProperty);
            }

            if (prop is UhtMapProperty mapProp)
            {
                info["key_prop"] = ExportInnerProperty(mapProp.KeyProperty);
                info["value_prop"] = ExportInnerProperty(mapProp.ValueProperty);
            }
        }

        private static bool ShouldExportFunction(UhtFunction func)
        {
            if (func.FunctionType != UhtFunctionType.Function)
                return false;

            if (func.FunctionFlags.HasAnyFlags(NoExportFuncFlags))
                return false;

            if (func.FunctionFlags.HasAnyFlags(EFunctionFlags.Protected)
                && !func.FunctionFlags.HasAnyFlags(EFunctionFlags.BlueprintCallable))
                return false;

            if (func.MetaData.ContainsKey("DeprecatedFunction"))
                return false;

            if (!func.FunctionFlags.HasAnyFlags(EFunctionFlags.Native)
                && !func.FunctionFlags.HasAnyFlags(EFunctionFlags.BlueprintEvent))
                return false;

            return true;
        }

        private static JsonObject ExportFunction(UhtFunction func)
        {
            var funcParams = new JsonArray();

            foreach (UhtType childType in func.ParameterProperties.Span)
            {
                if (childType is UhtProperty param)
                    funcParams.Add(ExportParam(param, func));
            }

            if (func.ReturnProperty != null)
                funcParams.Add(ExportParam(func.ReturnProperty, func));

            return new JsonObject
            {
                ["name"] = func.SourceName,
                ["func_flags"] = (long)unchecked((uint)func.FunctionFlags),
                ["is_static"] = func.FunctionFlags.HasAnyFlags(EFunctionFlags.Static),
                ["params"] = funcParams,
            };
        }

        private static JsonObject ExportDelegateSignature(UhtFunction func)
        {
            var funcParams = new JsonArray();

            foreach (UhtType childType in func.ParameterProperties.Span)
            {
                if (childType is UhtProperty param)
                    funcParams.Add(ExportParam(param, func));
            }

            if (func.ReturnProperty != null)
                funcParams.Add(ExportParam(func.ReturnProperty, func));

            return new JsonObject
            {
                ["name"] = func.StrippedFunctionName ?? func.SourceName,
                ["func_flags"] = (long)unchecked((uint)func.FunctionFlags),
                ["is_static"] = false,
                ["params"] = funcParams,
            };
        }

        private void WriteJsonFiles()
        {
            var writerOptions = new JsonWriterOptions
            {
                Indented = true,
                Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
            };

            WriteFile("rusteal_classes", new JsonObject
            {
                ["classes"] = ToJsonArray(_classes),
            }, writerOptions);

            WriteFile("rusteal_structs", new JsonObject
            {
                ["structs"] = ToJsonArray(_structs),
            }, writerOptions);

            WriteFile("rusteal_enums", new JsonObject
            {
                ["enums"] = ToJsonArray(_enums),
            }, writerOptions);
        }

        private void WriteFile(string name, JsonObject content, JsonWriterOptions writerOptions)
        {
            string path = _factory.MakePath(name, ".json");
            using var stream = new MemoryStream();

            using (var writer = new Utf8JsonWriter(stream, writerOptions))
            {
                content.WriteTo(writer);
            }

            string json = Encoding.UTF8.GetString(stream.ToArray());
            _factory.CommitOutput(path, new StringBuilder(json));
        }

        private static JsonArray ToJsonArray(List<JsonObject> items)
        {
            var arr = new JsonArray();

            foreach (var item in items)
                arr.Add(item);

            return arr;
        }

        private static string StripPrefix(string name)
        {
            if (name.Length <= 1) return name;

            if (name[0] is 'A' or 'U' or 'F' && char.IsUpper(name[1]))
                return name[1..];

            return name;
        }

        private static string GetPropertyTypeName(UhtProperty prop) => prop switch
        {
            UhtBoolProperty => "BoolProperty",
            UhtByteProperty => "ByteProperty",
            UhtInt8Property => "Int8Property",
            UhtInt16Property => "Int16Property",
            UhtIntProperty => "IntProperty",
            UhtInt64Property => "Int64Property",
            UhtUInt16Property => "UInt16Property",
            UhtUInt32Property => "UInt32Property",
            UhtUInt64Property => "UInt64Property",
            UhtFloatProperty => "FloatProperty",
            UhtDoubleProperty => "DoubleProperty",
            UhtLargeWorldCoordinatesRealProperty => "DoubleProperty",

            UhtStrProperty => "StrProperty",
            UhtNameProperty => "NameProperty",
            UhtTextProperty => "TextProperty",

            UhtEnumProperty => "EnumProperty",
            UhtStructProperty => "StructProperty",

            UhtClassProperty => "ClassProperty",
            UhtSoftClassProperty => "SoftClassProperty",
            UhtSoftObjectProperty => "SoftObjectProperty",
            UhtWeakObjectPtrProperty => "WeakObjectProperty",
            UhtLazyObjectPtrProperty => "LazyObjectProperty",
            UhtObjectProperty => "ObjectProperty",

            UhtInterfaceProperty => "InterfaceProperty",

            UhtDelegateProperty => "DelegateProperty",
            UhtMulticastInlineDelegateProperty => "MulticastInlineDelegateProperty",
            UhtMulticastSparseDelegateProperty => "MulticastSparseDelegateProperty",
            UhtMulticastDelegateProperty => "MulticastDelegateProperty",

            UhtArrayProperty => "ArrayProperty",
            UhtSetProperty => "SetProperty",
            UhtMapProperty => "MapProperty",

            UhtFieldPathProperty => "FieldPathProperty",

            _ => prop.EngineClassName,
        };

        private static int GetArrayDim(UhtProperty prop)
        {
            if (string.IsNullOrEmpty(prop.ArrayDimensions))
                return 1;

            return int.TryParse(prop.ArrayDimensions, out int dim) ? dim : 2;
        }

        private static string GetUnderlyingTypeName(UhtProperty prop) => prop switch
        {
            UhtByteProperty => "uint8",
            UhtInt8Property => "int8",
            UhtInt16Property => "int16",
            UhtUInt16Property => "uint16",
            UhtIntProperty => "int32",
            UhtUInt32Property => "uint32",
            UhtInt64Property => "int64",
            UhtUInt64Property => "uint64",
            _ => "uint8",
        };

        private static string GetMetaOrNull(UhtType type, string key)
        {
            return type.MetaData.TryGetValue(key, out string value) ? value : null;
        }
    }

    #endregion
}
