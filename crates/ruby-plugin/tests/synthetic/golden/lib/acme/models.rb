# frozen_string_literal: true

module Acme
  [Acme::Internal::Type::BaseModel, *Acme::Internal::Type::BaseModel.subclasses].each do |cls|
    cls.define_sorbet_constant!(:OrHash) { T.type_alias { T.any(cls, Acme::Internal::AnyHash) } }
  end

  Acme::Internal::Util.walk_namespaces(Acme::Models).each do |mod|
    case mod
    in Acme::Internal::Type::Enum | Acme::Internal::Type::Union
      mod.constants.each do |name|
        case mod.const_get(name)
        in true | false
          mod.define_sorbet_constant!(:TaggedBoolean) { T.type_alias { T::Boolean } }
          mod.define_sorbet_constant!(:OrBoolean) { T.type_alias { T::Boolean } }
        in Integer
          mod.define_sorbet_constant!(:TaggedInteger) { T.type_alias { Integer } }
          mod.define_sorbet_constant!(:OrInteger) { T.type_alias { Integer } }
        in Float
          mod.define_sorbet_constant!(:TaggedFloat) { T.type_alias { Float } }
          mod.define_sorbet_constant!(:OrFloat) { T.type_alias { Float } }
        in Symbol
          mod.define_sorbet_constant!(:TaggedSymbol) { T.type_alias { Symbol } }
          mod.define_sorbet_constant!(:OrSymbol) { T.type_alias { T.any(Symbol, String) } }
        else
        end
      end
    else
    end
  end

  Acme::Internal::Util.walk_namespaces(Acme::Models)
                      .lazy
                      .grep(Acme::Internal::Type::Union)
                      .each do |mod|
    const = :Variants
    next if mod.sorbet_constant_defined?(const)

    mod.define_sorbet_constant!(const) { T.type_alias { mod.to_sorbet_type } }
  end


  ErrorInfo = Acme::Models::ErrorInfo

  # @type [Acme::Internal::Type::Converter]
  Labels = Acme::Models::Labels

  LabelsItem = Acme::Models::LabelsItem

  Widget = Acme::Models::Widget

  WidgetCreateParams = Acme::Models::WidgetCreateParams

  WidgetDeleteParams = Acme::Models::WidgetDeleteParams

  WidgetListParams = Acme::Models::WidgetListParams

  WidgetRetrieveParams = Acme::Models::WidgetRetrieveParams

  Widgets = Acme::Models::Widgets
end
