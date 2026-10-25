# frozen_string_literal: true

module Acme
  module Models
    class LabelsItem < Acme::Internal::Type::BaseModel
      # @!attribute text
      #
      #   @return [String]
      required :text, String

      # @!attribute color
      #
      #   @return [String, nil]
      optional :color, String

      # @!method initialize(text:, color: nil)
      #   @param text [String]
      #   @param color [String]
    end

    # @type [Acme::Internal::Type::Converter]
    Labels = Acme::Internal::Type::HashOf[-> { Acme::LabelsItem }]
  end
end
