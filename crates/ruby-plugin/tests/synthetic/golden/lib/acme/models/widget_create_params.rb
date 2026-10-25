# frozen_string_literal: true

module Acme
  module Models
    # @see Acme::Resources::Widgets#create
    class WidgetCreateParams < Acme::Internal::Type::BaseModel
      extend Acme::Internal::Type::RequestParameters::Converter
      include Acme::Internal::Type::RequestParameters

      # @!attribute name
      #   The widget's display name.
      #
      #   @return [String]
      required :name, String

      # @!attribute color
      #
      #   @return [Symbol, Acme::Models::WidgetCreateParams::Color, nil]
      optional :color, enum: -> { Acme::WidgetCreateParams::Color }

      # @!attribute dimensions
      #
      #   @return [Acme::Models::WidgetCreateParams::Dimensions, nil]
      optional :dimensions, -> { Acme::WidgetCreateParams::Dimensions }

      # @!attribute tags
      #
      #   @return [Array<String>, nil]
      optional :tags, Acme::Internal::Type::ArrayOf[String]

      # @!method initialize(name:, color: nil, dimensions: nil, tags: nil, request_options: {})
      #   @param name [String] The widget's display name.
      #
      #   @param color [Symbol, Acme::Models::WidgetCreateParams::Color]
      #
      #   @param dimensions [Acme::Models::WidgetCreateParams::Dimensions]
      #
      #   @param tags [Array<String>]
      #
      #   @param request_options [Acme::RequestOptions, Hash{Symbol=>Object}]

      module Color
        extend Acme::Internal::Type::Enum

        RED = :red
        GREEN = :green
        BLUE = :blue

        # @!method self.values
        #   @return [Array<Symbol>]
      end

      class Dimensions < Acme::Internal::Type::BaseModel
        # @!attribute width
        #
        #   @return [Float]
        required :width, Float

        # @!attribute height
        #
        #   @return [Float, nil]
        optional :height, Float

        # @!method initialize(width:, height: nil)
        #   @param width [Float]
        #   @param height [Float]
      end
    end
  end
end
