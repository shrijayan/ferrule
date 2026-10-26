# typed: strong

module Acme
  ErrorInfo = Acme::Models::ErrorInfo

  Labels = T.let(Acme::Models::Labels, Acme::Internal::Type::Converter)

  LabelsItem = Acme::Models::LabelsItem

  Widget = Acme::Models::Widget

  WidgetCreateParams = Acme::Models::WidgetCreateParams

  WidgetDeleteParams = Acme::Models::WidgetDeleteParams

  WidgetListParams = Acme::Models::WidgetListParams

  WidgetRetrieveParams = Acme::Models::WidgetRetrieveParams

  Widgets = Acme::Models::Widgets
end
