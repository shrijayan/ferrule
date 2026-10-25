# frozen_string_literal: true

module Acme
  module Models
    module Widgets
      # @see Acme::Resources::Widgets::Parts#retrieve
      class PartRetrieveParams < Acme::Internal::Type::BaseModel
        extend Acme::Internal::Type::RequestParameters::Converter
        include Acme::Internal::Type::RequestParameters

        # @!attribute widget_id
        #
        #   @return [String]
        required :widget_id, String

        # @!attribute part_number
        #
        #   @return [Integer]
        required :part_number, Integer

        # @!method initialize(widget_id:, part_number:, request_options: {})
        #   @param widget_id [String]
        #   @param part_number [Integer]
        #   @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}]
      end
    end
  end
end
