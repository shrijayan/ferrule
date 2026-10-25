# frozen_string_literal: true

module Acme
  module Models
    module Widgets
      # @see Acme::Resources::Widgets::Parts#retrieve
      class PartRetrieveResponse < Acme::Internal::Type::BaseModel
        # @!attribute id
        #
        #   @return [String]
        required :id, String

        # @!attribute sku
        #   The part's stock keeping unit.
        #
        #   @return [String]
        required :sku, String

        # @!attribute error
        #
        #   @return [Acme::Models::ErrorInfo, nil]
        optional :error, -> { Acme::ErrorInfo }

        # @!method initialize(id:, sku:, error: nil)
        #   @param id [String]
        #
        #   @param sku [String] The part's stock keeping unit.
        #
        #   @param error [Acme::Models::ErrorInfo]
      end
    end
  end
end
