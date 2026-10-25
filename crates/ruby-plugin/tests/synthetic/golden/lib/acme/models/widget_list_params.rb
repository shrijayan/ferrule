# frozen_string_literal: true

module Acme
  module Models
    # @see Acme::Resources::Widgets#list
    class WidgetListParams < Acme::Internal::Type::BaseModel
      extend Acme::Internal::Type::RequestParameters::Converter
      include Acme::Internal::Type::RequestParameters

      # @!attribute colors
      #
      #   @return [Array<String>, nil]
      optional :colors, Acme::Internal::Type::ArrayOf[String]

      # @!attribute created_after
      #
      #   @return [Time, nil]
      optional :created_after, Time

      # @!attribute limit
      #
      #   @return [Integer, nil]
      optional :limit, Integer

      # @!method initialize(colors: nil, created_after: nil, limit: nil, request_options: {})
      #   @param colors [Array<String>]
      #   @param created_after [Time]
      #   @param limit [Integer]
      #   @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}]
    end
  end
end
