# frozen_string_literal: true

module Acme
  module Models
    # @see Acme::Resources::Widgets#delete
    class WidgetDeleteParams < Acme::Internal::Type::BaseModel
      extend Acme::Internal::Type::RequestParameters::Converter
      include Acme::Internal::Type::RequestParameters

      # @!attribute widget_id
      #
      #   @return [String]
      required :widget_id, String

      # @!method initialize(widget_id:, request_options: {})
      #   @param widget_id [String]
      #   @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}]
    end
  end
end
